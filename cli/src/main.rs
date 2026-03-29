//! cor24-dbg: GDB-like CLI debugger for the MakerLisp COR24 processor
//!
//! Usage:
//!   cor24-dbg <file.lgo>           Load an LGO file and start debugging
//!   cor24-dbg --entry 0x93 <file>  Set entry point address

use cor24_emulator::emulator::{EmulatorCore, StopReason};
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("cor24-dbg: MakerLisp COR24 debugger\n");
        eprintln!("Usage:");
        eprintln!("  cor24-dbg <file.lgo>                Load LGO file");
        eprintln!("  cor24-dbg --entry <addr> <file.lgo>  Set entry point");
        eprintln!("\nCommands: run, step, break, info, examine, disas, reset, quit");
        std::process::exit(1);
    }

    let mut entry_override: Option<u32> = None;
    let mut filename = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--entry" | "-e" => {
                i += 1;
                if i < args.len() {
                    entry_override = Some(parse_addr(&args[i]).unwrap_or_else(|| {
                        eprintln!("Bad address: {}", args[i]);
                        std::process::exit(1);
                    }));
                }
            }
            _ => {
                filename = Some(args[i].clone());
            }
        }
        i += 1;
    }

    let filename = filename.unwrap_or_else(|| {
        eprintln!("No file specified");
        std::process::exit(1);
    });

    let mut dbg = Debugger::new();

    if let Err(e) = dbg.load_file(&filename, entry_override) {
        eprintln!("Error loading {}: {}", filename, e);
        std::process::exit(1);
    }

    dbg.repl();
}

struct Debugger {
    emu: EmulatorCore,
    loaded: bool,
    /// Track UART output position for incremental printing
    uart_printed: usize,
}

impl Debugger {
    fn new() -> Self {
        Self {
            emu: EmulatorCore::new(),
            loaded: false,
            uart_printed: 0,
        }
    }

    fn load_file(&mut self, path: &str, entry: Option<u32>) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Cannot read file: {}", e))?;

        let bytes = self.emu.load_lgo(&content, entry)?;
        self.loaded = true;
        self.uart_printed = 0;

        println!("Loaded {} bytes from {}", bytes, path);
        println!("PC = 0x{:06X}", self.emu.pc());
        Ok(())
    }

    fn repl(&mut self) {
        let stdin = io::stdin();
        let mut last_cmd = String::new();

        loop {
            print!("(cor24) ");
            io::stdout().flush().ok();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
                println!();
                break;
            }
            let line = line.trim().to_string();

            let cmd = if line.is_empty() {
                last_cmd.clone()
            } else {
                last_cmd = line.clone();
                line
            };

            if cmd.is_empty() {
                continue;
            }

            let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
            let arg = if parts.len() > 1 { parts[1].trim() } else { "" };

            match parts[0] {
                "q" | "quit" | "exit" => break,
                "r" | "run" => self.cmd_run(arg),
                "s" | "step" | "si" => self.cmd_step(arg),
                "n" | "next" => self.cmd_next(),
                "c" | "cont" | "continue" => self.cmd_continue(),
                "b" | "break" => self.cmd_break(arg),
                "d" | "delete" => self.cmd_delete(arg),
                "i" | "info" => self.cmd_info(arg),
                "x" | "examine" => self.cmd_examine(arg),
                "p" | "print" => self.cmd_print(arg),
                "disas" | "disassemble" => self.cmd_disas(arg),
                "load" => {
                    if arg.is_empty() {
                        println!("Usage: load <file.lgo>");
                    } else if let Err(e) = self.load_file(arg, None) {
                        println!("Error: {}", e);
                    }
                }
                "reset" => {
                    self.emu.reset();
                    self.uart_printed = 0;
                    println!("CPU reset. PC = 0x{:06X}", self.emu.pc());
                }
                "t" | "trace" => self.cmd_trace(arg),
                "uart" => self.cmd_uart(arg),
                "led" => self.cmd_led(),
                "button" | "btn" => self.cmd_button(arg),
                "help" | "h" | "?" => self.cmd_help(),
                _ => println!("Unknown command: '{}'. Type 'help' for commands.", parts[0]),
            }
        }
    }

    fn print_new_uart(&mut self) {
        let output = self.emu.get_uart_output();
        if output.len() > self.uart_printed {
            print!("{}", &output[self.uart_printed..]);
            io::stdout().flush().ok();
            self.uart_printed = output.len();
        }
    }

    fn cmd_run(&mut self, arg: &str) {
        if self.emu.is_halted() {
            println!("CPU is halted. Use 'reset' to restart.");
            return;
        }

        let max: u64 = if arg.is_empty() {
            100_000_000
        } else {
            arg.replace('_', "").parse().unwrap_or(100_000_000)
        };

        self.emu.resume();
        let result = self.emu.run_batch(max);
        self.print_new_uart();

        match result.reason {
            StopReason::Halted => {
                println!(
                    "\nCPU halted after {} instructions",
                    result.instructions_run
                );
                self.cmd_trace("10");
            }
            StopReason::Breakpoint(addr) => {
                println!("Breakpoint at 0x{:06X}", addr);
            }
            StopReason::InvalidInstruction(byte) => {
                println!(
                    "\nInvalid instruction 0x{:02X} after {} instructions",
                    byte, result.instructions_run
                );
            }
            StopReason::CycleLimit => {
                println!(
                    "\nStopped after {} instructions (limit). PC = 0x{:06X}",
                    result.instructions_run,
                    self.emu.pc()
                );
            }
            StopReason::Paused => {}
        }

        self.show_location();
    }

    fn cmd_step(&mut self, arg: &str) {
        let n: u64 = if arg.is_empty() {
            1
        } else {
            arg.parse().unwrap_or(1)
        };

        for _ in 0..n {
            if self.emu.is_halted() {
                println!("CPU is halted.");
                return;
            }
            self.emu.step();
        }

        self.print_new_uart();
        self.show_location();
        self.show_regs_short();
    }

    fn cmd_next(&mut self) {
        if self.emu.is_halted() {
            println!("CPU is halted.");
            return;
        }
        self.emu.step_over();
        self.print_new_uart();
        self.show_location();
        self.show_regs_short();
    }

    fn cmd_continue(&mut self) {
        if self.emu.is_halted() {
            println!("CPU is halted.");
            return;
        }

        self.emu.resume();
        let result = self.emu.run_batch(100_000_000);
        self.print_new_uart();

        match result.reason {
            StopReason::Breakpoint(addr) => println!("Breakpoint at 0x{:06X}", addr),
            StopReason::Halted => {
                println!("\nHalted after {} instructions", result.instructions_run)
            }
            _ => {}
        }

        self.show_location();
    }

    fn cmd_break(&mut self, arg: &str) {
        if arg.is_empty() {
            println!("Usage: break <address>");
            return;
        }
        if let Some(addr) = parse_addr(arg) {
            self.emu.add_breakpoint(addr);
            println!(
                "Breakpoint {} at 0x{:06X}",
                self.emu.breakpoints().len(),
                addr
            );
        } else {
            println!("Bad address: {}", arg);
        }
    }

    fn cmd_delete(&mut self, arg: &str) {
        if arg.is_empty() || arg == "all" {
            self.emu.clear_breakpoints();
            println!("All breakpoints deleted.");
        } else if let Ok(n) = arg.parse::<usize>() {
            if n >= 1 {
                if let Some(addr) = self.emu.remove_breakpoint_by_index(n - 1) {
                    println!("Deleted breakpoint {} at 0x{:06X}", n, addr);
                } else {
                    println!("No breakpoint #{}", n);
                }
            }
        } else {
            println!("Usage: delete <N> or delete all");
        }
    }

    fn cmd_info(&self, arg: &str) {
        match arg {
            "r" | "reg" | "registers" => self.show_regs(),
            "t" | "trace" => self.cmd_trace("20"),
            "b" | "break" | "breakpoints" => {
                let bps = self.emu.breakpoints();
                if bps.is_empty() {
                    println!("No breakpoints.");
                } else {
                    for (i, &addr) in bps.iter().enumerate() {
                        println!("  #{}: 0x{:06X}", i + 1, addr);
                    }
                }
            }
            "" => self.show_regs(),
            _ => println!("info: r(egisters), t(race), b(reakpoints)"),
        }
    }

    fn cmd_examine(&self, arg: &str) {
        let (count, addr_str) = if let Some(rest) = arg.strip_prefix('/') {
            if let Some(space) = rest.find(' ') {
                let n: usize = rest[..space].parse().unwrap_or(16);
                (n, rest[space..].trim())
            } else {
                (16, rest)
            }
        } else {
            (16, arg)
        };

        if addr_str.is_empty() {
            println!("Usage: x [/N] <address>");
            return;
        }

        let addr = match parse_addr(addr_str) {
            Some(a) => a,
            None => {
                println!("Bad address: {}", addr_str);
                return;
            }
        };

        let bytes = self.emu.read_memory(addr, count as u32);
        let mut offset = 0;
        while offset < bytes.len() {
            let row_len = std::cmp::min(16, bytes.len() - offset);
            print!("0x{:06X}:", addr + offset as u32);
            for i in 0..row_len {
                print!(" {:02X}", bytes[offset + i]);
            }
            print!("  |");
            for i in 0..row_len {
                let b = bytes[offset + i];
                if (0x20..0x7F).contains(&b) {
                    print!("{}", b as char);
                } else {
                    print!(".");
                }
            }
            println!("|");
            offset += row_len;
        }
    }

    fn cmd_print(&self, arg: &str) {
        match arg.to_lowercase().as_str() {
            "r0" => println!(
                "r0 = 0x{:06X} ({})",
                self.emu.get_reg(0),
                self.emu.get_reg(0) as i32
            ),
            "r1" => println!(
                "r1 = 0x{:06X} ({})",
                self.emu.get_reg(1),
                self.emu.get_reg(1) as i32
            ),
            "r2" => println!(
                "r2 = 0x{:06X} ({})",
                self.emu.get_reg(2),
                self.emu.get_reg(2) as i32
            ),
            "fp" | "r3" => println!("fp = 0x{:06X}", self.emu.get_reg(3)),
            "sp" | "r4" => println!("sp = 0x{:06X}", self.emu.get_reg(4)),
            "pc" => println!("pc = 0x{:06X}", self.emu.pc()),
            "c" => println!("c = {}", self.emu.condition_flag()),
            "led" | "leds" => println!(
                "LED = 0x{:02X} (bit0={})",
                self.emu.get_led(),
                self.emu.get_led() & 1
            ),
            _ => {
                if let Some(addr) = parse_addr(arg) {
                    println!("[0x{:06X}] = 0x{:02X}", addr, self.emu.read_byte(addr));
                } else {
                    println!("Usage: print <register|address>");
                }
            }
        }
    }

    fn cmd_disas(&self, arg: &str) {
        let parts: Vec<&str> = arg.split_whitespace().collect();
        let addr = if parts.is_empty() {
            self.emu.pc()
        } else {
            parse_addr(parts[0]).unwrap_or(self.emu.pc())
        };
        let count: usize = if parts.len() > 1 {
            parts[1].parse().unwrap_or(10)
        } else {
            10
        };

        for (pc, text, size) in self.emu.disassemble(addr, count) {
            let marker = if pc == self.emu.pc() { "=> " } else { "   " };
            let bp = if self.emu.has_breakpoint(pc) {
                "*"
            } else {
                " "
            };
            let mut bytes_str = String::new();
            for i in 0..size {
                bytes_str.push_str(&format!("{:02X} ", self.emu.read_byte(pc + i)));
            }
            println!("{}{}{:06X}: {:12} {}", bp, marker, pc, bytes_str, text);
        }
    }

    fn cmd_trace(&self, arg: &str) {
        let n: usize = if arg.is_empty() {
            20
        } else {
            arg.parse().unwrap_or(20)
        };
        let trace = self.emu.trace();
        let entries = trace.last_n(n);
        if entries.is_empty() {
            println!("No trace entries.");
        } else {
            for entry in &entries {
                println!("{}", entry);
            }
            println!("({} of {} entries shown)", entries.len(), trace.len());
        }
    }

    fn cmd_uart(&mut self, arg: &str) {
        if let Some(rest) = arg
            .strip_prefix("send ")
            .or_else(|| arg.strip_prefix("tx "))
        {
            let rest = rest.trim();
            // Support: single char, decimal, 0x hex, or quoted string
            if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
                let s = &rest[1..rest.len() - 1];
                for b in s.bytes() {
                    self.emu.send_uart_byte(b);
                }
                println!("Sent {} byte(s) to UART RX", s.len());
            } else if let Some(byte) = parse_byte_value(rest) {
                self.emu.send_uart_byte(byte);
                println!("Sent 0x{:02X} ({}) to UART RX", byte, byte);
            } else {
                println!("Usage: uart send <byte|0xHH|\"string\">");
            }
        } else {
            let output = self.emu.get_uart_output();
            println!("UART output buffer ({} chars):", output.len());
            println!("{}", output);
        }
    }

    fn cmd_led(&self) {
        let led = self.emu.get_led() & 1;
        let pressed = self.emu.get_button();
        println!(
            "LED D2: {} (bit0 = {})",
            if led != 0 { "ON" } else { "OFF" },
            led
        );
        println!(
            "Button S2: {} (bit0 = {})",
            if pressed { "LOW (pressed)" } else { "HIGH" },
            if pressed { 0 } else { 1 }
        );
    }

    fn cmd_button(&mut self, arg: &str) {
        match arg {
            "press" | "down" | "1" => {
                self.emu.set_button_pressed(true);
                println!("Button S2: pressed (LOW)");
            }
            "release" | "up" | "0" => {
                self.emu.set_button_pressed(false);
                println!("Button S2: released (HIGH)");
            }
            "toggle" | "t" => {
                let currently = self.emu.get_button();
                self.emu.set_button_pressed(!currently);
                if !currently {
                    println!("Button S2: pressed (LOW)");
                } else {
                    println!("Button S2: released (HIGH)");
                }
            }
            "" => {
                let pressed = self.emu.get_button();
                println!(
                    "Button S2: {}",
                    if pressed {
                        "pressed (LOW)"
                    } else {
                        "released (HIGH)"
                    }
                );
            }
            _ => println!("Usage: button [press|release|toggle]"),
        }
    }

    fn cmd_help(&self) {
        println!("Commands:");
        println!("  r, run [N]          Run N instructions (default 100M)");
        println!("  s, step [N]         Single step (N instructions)");
        println!("  n, next             Step over (skip jal calls)");
        println!("  c, continue         Continue from breakpoint");
        println!("  b, break <addr>     Set breakpoint");
        println!("  d, delete <N|all>   Delete breakpoint(s)");
        println!("  i, info [r|b]       Show registers or breakpoints");
        println!("  x [/N] <addr>       Examine N bytes at address");
        println!("  p, print <reg|addr> Print register or memory");
        println!("  t, trace [N]        Show last N traced instructions (default 20)");
        println!("  disas [addr] [N]    Disassemble N instructions");
        println!("  load <file.lgo>     Load LGO file");
        println!("  uart                Show UART output buffer");
        println!("  uart send <val>     Send byte to UART RX (decimal, 0xHH, or \"string\")");
        println!("  led                 Show LED/button state");
        println!("  button [press|release|toggle]  Control button S2");
        println!("  reset               Reset CPU");
        println!("  q, quit             Exit");
        println!();
        println!("Addresses: decimal, 0x-hex, or 0b-binary");
        println!("Empty line repeats last command.");
    }

    fn show_location(&self) {
        let (text, _) = self.emu.disassemble_at(self.emu.pc());
        println!("0x{:06X}: {}", self.emu.pc(), text);
    }

    fn show_regs(&self) {
        let s = self.emu.snapshot();
        println!(
            "  r0 = 0x{:06X}  r1 = 0x{:06X}  r2 = 0x{:06X}",
            s.regs[0], s.regs[1], s.regs[2]
        );
        println!(
            "  fp = 0x{:06X}  sp = 0x{:06X}  z  = 0x{:06X}",
            s.regs[3], s.regs[4], s.regs[5]
        );
        println!("  iv = 0x{:06X}  ir = 0x{:06X}", s.regs[6], s.regs[7]);
        println!("  pc = 0x{:06X}  c  = {}", s.pc, s.c as u8);
        println!("  LED = 0x{:02X}  cycles = {}", s.led, s.cycles);
    }

    fn show_regs_short(&self) {
        let s = self.emu.snapshot();
        println!(
            "  r0={:06X} r1={:06X} r2={:06X} fp={:06X} sp={:06X} c={}",
            s.regs[0], s.regs[1], s.regs[2], s.regs[3], s.regs[4], s.c as u8
        );
    }
}

fn parse_byte_value(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.len() == 1 {
        return Some(s.as_bytes()[0]);
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

fn parse_addr(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = s.strip_prefix("0b") {
        u32::from_str_radix(bin, 2).ok()
    } else {
        s.parse().ok()
    }
}
