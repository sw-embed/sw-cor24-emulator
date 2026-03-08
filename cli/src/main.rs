//! cor24-dbg: GDB-like CLI debugger for the MakerLisp COR24 processor
//!
//! Usage:
//!   cor24-dbg <file.lgo>           Load an LGO file and start debugging
//!   cor24-dbg --entry 0x93 <file>  Set entry point address

use cor24_emulator::cpu::decode_rom::DECODE_ROM;
use cor24_emulator::cpu::executor::Executor;
use cor24_emulator::cpu::state::CpuState;
use cor24_emulator::loader::load_lgo;
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
    cpu: CpuState,
    executor: Executor,
    breakpoints: Vec<u32>,
    loaded: bool,
}

impl Debugger {
    fn new() -> Self {
        Self {
            cpu: CpuState::new(),
            executor: Executor::new(),
            breakpoints: Vec::new(),
            loaded: false,
        }
    }

    fn load_file(&mut self, path: &str, entry: Option<u32>) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read file: {}", e))?;

        self.cpu = CpuState::new();
        let result = load_lgo(&content, &mut self.cpu)?;

        let start = entry
            .or(result.start_addr)
            .unwrap_or(0);
        self.cpu.pc = start;
        self.loaded = true;

        println!("Loaded {} bytes from {}", result.bytes_loaded, path);
        println!("PC = 0x{:06X}", self.cpu.pc);
        Ok(())
    }

    fn repl(&mut self) {
        let stdin = io::stdin();
        let mut last_cmd = String::new();

        loop {
            // Flush any UART output
            self.flush_uart();

            print!("(cor24) ");
            io::stdout().flush().ok();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
                println!();
                break;
            }
            let line = line.trim().to_string();

            // Empty line repeats last command
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
                    self.cpu.reset();
                    println!("CPU reset. PC = 0x{:06X}", self.cpu.pc);
                }
                "uart" => {
                    println!("UART output buffer ({} chars):", self.cpu.io.uart_output.len());
                    println!("{}", self.cpu.io.uart_output);
                }
                "led" => self.cmd_led(),
                "help" | "h" | "?" => self.cmd_help(),
                _ => println!("Unknown command: '{}'. Type 'help' for commands.", parts[0]),
            }
        }
    }

    fn flush_uart(&mut self) {
        // Print any new UART output since last flush
        // We track this by keeping the output buffer and printing it
        // The caller can use 'uart' command to see full buffer
    }

    fn cmd_run(&mut self, arg: &str) {
        if self.cpu.halted {
            println!("CPU is halted. Use 'reset' to restart.");
            return;
        }

        let uart_before = self.cpu.io.uart_output.len();
        let mut count = 0u64;
        let max: u64 = if arg.is_empty() {
            100_000_000
        } else {
            arg.replace('_', "").parse().unwrap_or(100_000_000)
        };

        loop {
            if self.cpu.halted || count >= max {
                break;
            }
            if count > 0 && self.breakpoints.contains(&self.cpu.pc) {
                println!("Breakpoint at 0x{:06X}", self.cpu.pc);
                break;
            }
            self.executor.step(&mut self.cpu);
            count += 1;
        }

        // Print UART output produced during run
        let new_output = &self.cpu.io.uart_output[uart_before..];
        if !new_output.is_empty() {
            print!("{}", new_output);
            io::stdout().flush().ok();
        }

        if self.cpu.halted {
            println!("\nCPU halted after {} instructions", count);
        } else if count >= max {
            println!("\nStopped after {} instructions (limit). PC = 0x{:06X}", count, self.cpu.pc);
        }

        self.show_location();
    }

    fn cmd_step(&mut self, arg: &str) {
        let n: u64 = if arg.is_empty() {
            1
        } else {
            arg.parse().unwrap_or(1)
        };

        let uart_before = self.cpu.io.uart_output.len();

        for _ in 0..n {
            if self.cpu.halted {
                println!("CPU is halted.");
                return;
            }
            self.executor.step(&mut self.cpu);
        }

        let new_output = &self.cpu.io.uart_output[uart_before..];
        if !new_output.is_empty() {
            print!("{}", new_output);
            io::stdout().flush().ok();
        }

        self.show_location();
        self.show_regs_short();
    }

    fn cmd_next(&mut self) {
        if self.cpu.halted {
            println!("CPU is halted.");
            return;
        }

        // Check if current instruction is jal (call)
        let byte0 = self.cpu.read_byte(self.cpu.pc);
        let decoded = DECODE_ROM[byte0 as usize];
        let opcode = (decoded >> 6) & 0x1F;

        if opcode == 0x09 {
            // JAL — step over: run until PC is past this instruction
            let return_pc = self.cpu.pc + 1; // jal is 1 byte
            let uart_before = self.cpu.io.uart_output.len();
            let mut count = 0u64;

            self.executor.step(&mut self.cpu);
            count += 1;

            while self.cpu.pc != return_pc && !self.cpu.halted && count < 10_000_000 {
                self.executor.step(&mut self.cpu);
                count += 1;
            }

            let new_output = &self.cpu.io.uart_output[uart_before..];
            if !new_output.is_empty() {
                print!("{}", new_output);
                io::stdout().flush().ok();
            }
        } else {
            self.executor.step(&mut self.cpu);
        }

        self.show_location();
        self.show_regs_short();
    }

    fn cmd_continue(&mut self) {
        if self.cpu.halted {
            println!("CPU is halted.");
            return;
        }

        let uart_before = self.cpu.io.uart_output.len();
        let mut count = 0u64;
        let max = 100_000_000u64;

        loop {
            if self.cpu.halted || count >= max {
                break;
            }
            self.executor.step(&mut self.cpu);
            count += 1;
            if self.breakpoints.contains(&self.cpu.pc) {
                println!("Breakpoint at 0x{:06X}", self.cpu.pc);
                break;
            }
        }

        let new_output = &self.cpu.io.uart_output[uart_before..];
        if !new_output.is_empty() {
            print!("{}", new_output);
            io::stdout().flush().ok();
        }

        if self.cpu.halted {
            println!("\nHalted after {} instructions", count);
        }

        self.show_location();
    }

    fn cmd_break(&mut self, arg: &str) {
        if arg.is_empty() {
            println!("Usage: break <address>");
            return;
        }
        if let Some(addr) = parse_addr(arg) {
            if !self.breakpoints.contains(&addr) {
                self.breakpoints.push(addr);
            }
            println!("Breakpoint {} at 0x{:06X}", self.breakpoints.len(), addr);
        } else {
            println!("Bad address: {}", arg);
        }
    }

    fn cmd_delete(&mut self, arg: &str) {
        if arg.is_empty() || arg == "all" {
            self.breakpoints.clear();
            println!("All breakpoints deleted.");
        } else if let Ok(n) = arg.parse::<usize>() {
            if n >= 1 && n <= self.breakpoints.len() {
                let addr = self.breakpoints.remove(n - 1);
                println!("Deleted breakpoint {} at 0x{:06X}", n, addr);
            } else {
                println!("No breakpoint #{}", n);
            }
        } else {
            println!("Usage: delete <N> or delete all");
        }
    }

    fn cmd_info(&self, arg: &str) {
        match arg {
            "r" | "reg" | "registers" => self.show_regs(),
            "b" | "break" | "breakpoints" => {
                if self.breakpoints.is_empty() {
                    println!("No breakpoints.");
                } else {
                    for (i, &addr) in self.breakpoints.iter().enumerate() {
                        println!("  #{}: 0x{:06X}", i + 1, addr);
                    }
                }
            }
            "" => {
                self.show_regs();
            }
            _ => println!("info: r(egisters), b(reakpoints)"),
        }
    }

    fn cmd_examine(&self, arg: &str) {
        // x/<N> <addr> or x <addr>
        let (count, addr_str) = if arg.starts_with('/') {
            let rest = &arg[1..];
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

        // Print in rows of 16
        let mut a = addr;
        let end = addr + count as u32;
        while a < end {
            print!("0x{:06X}:", a);
            let row_end = std::cmp::min(a + 16, end);
            for i in a..row_end {
                print!(" {:02X}", self.cpu.read_byte(i));
            }
            // ASCII
            print!("  |");
            for i in a..row_end {
                let b = self.cpu.read_byte(i);
                if b >= 0x20 && b < 0x7F {
                    print!("{}", b as char);
                } else {
                    print!(".");
                }
            }
            println!("|");
            a = row_end;
        }
    }

    fn cmd_print(&self, arg: &str) {
        match arg.to_lowercase().as_str() {
            "r0" => println!("r0 = 0x{:06X} ({})", self.cpu.get_reg(0), self.cpu.get_reg(0) as i32),
            "r1" => println!("r1 = 0x{:06X} ({})", self.cpu.get_reg(1), self.cpu.get_reg(1) as i32),
            "r2" => println!("r2 = 0x{:06X} ({})", self.cpu.get_reg(2), self.cpu.get_reg(2) as i32),
            "fp" | "r3" => println!("fp = 0x{:06X}", self.cpu.get_reg(3)),
            "sp" | "r4" => println!("sp = 0x{:06X}", self.cpu.get_reg(4)),
            "pc" => println!("pc = 0x{:06X}", self.cpu.pc),
            "c" => println!("c = {}", self.cpu.c),
            "led" | "leds" => println!("LED = 0x{:02X} (bit0={})", self.cpu.io.leds, self.cpu.io.leds & 1),
            _ => {
                if let Some(addr) = parse_addr(arg) {
                    println!("[0x{:06X}] = 0x{:02X}", addr, self.cpu.read_byte(addr));
                } else {
                    println!("Usage: print <register|address>");
                }
            }
        }
    }

    fn cmd_disas(&self, arg: &str) {
        let parts: Vec<&str> = arg.split_whitespace().collect();
        let addr = if parts.is_empty() {
            self.cpu.pc
        } else {
            parse_addr(parts[0]).unwrap_or(self.cpu.pc)
        };
        let count: usize = if parts.len() > 1 {
            parts[1].parse().unwrap_or(10)
        } else {
            10
        };

        let mut pc = addr;
        for _ in 0..count {
            let (text, size) = disassemble_at(&self.cpu, pc);
            let marker = if pc == self.cpu.pc { "=> " } else { "   " };
            let bp = if self.breakpoints.contains(&pc) { "*" } else { " " };

            // Show bytes
            let mut bytes_str = String::new();
            for i in 0..size {
                bytes_str.push_str(&format!("{:02X} ", self.cpu.read_byte(pc + i as u32)));
            }

            println!("{}{}{:06X}: {:12} {}", bp, marker, pc, bytes_str, text);
            pc += size as u32;
        }
    }

    fn cmd_led(&self) {
        let led = self.cpu.io.leds & 1;
        let btn = self.cpu.io.switches & 1;
        println!("LED D2: {} (bit0 = {})", if led != 0 { "ON" } else { "OFF" }, led);
        println!("Button S2: {} (bit0 = {})", if btn != 0 { "HIGH" } else { "LOW (pressed)" }, btn);
    }

    fn cmd_help(&self) {
        println!("Commands:");
        println!("  r, run              Run until halt/breakpoint");
        println!("  s, step [N]         Single step (N instructions)");
        println!("  n, next             Step over (skip jal calls)");
        println!("  c, continue         Continue from breakpoint");
        println!("  b, break <addr>     Set breakpoint");
        println!("  d, delete <N|all>   Delete breakpoint(s)");
        println!("  i, info [r|b]       Show registers or breakpoints");
        println!("  x [/N] <addr>       Examine N bytes at address");
        println!("  p, print <reg|addr> Print register or memory");
        println!("  disas [addr] [N]    Disassemble N instructions");
        println!("  load <file.lgo>     Load LGO file");
        println!("  uart                Show UART output buffer");
        println!("  led                 Show LED/button state");
        println!("  reset               Reset CPU");
        println!("  q, quit             Exit");
        println!();
        println!("Addresses: decimal, 0x-hex, or 0b-binary");
        println!("Empty line repeats last command.");
    }

    fn show_location(&self) {
        let (text, _) = disassemble_at(&self.cpu, self.cpu.pc);
        println!("0x{:06X}: {}", self.cpu.pc, text);
    }

    fn show_regs(&self) {
        println!("  r0 = 0x{:06X}  r1 = 0x{:06X}  r2 = 0x{:06X}",
            self.cpu.get_reg(0), self.cpu.get_reg(1), self.cpu.get_reg(2));
        println!("  fp = 0x{:06X}  sp = 0x{:06X}  z  = 0x{:06X}",
            self.cpu.get_reg(3), self.cpu.get_reg(4), self.cpu.get_reg(5));
        println!("  iv = 0x{:06X}  ir = 0x{:06X}",
            self.cpu.get_reg(6), self.cpu.get_reg(7));
        println!("  pc = 0x{:06X}  c  = {}", self.cpu.pc, self.cpu.c as u8);
        println!("  LED = 0x{:02X}  cycles = {}", self.cpu.io.leds, self.cpu.cycles);
    }

    fn show_regs_short(&self) {
        println!("  r0={:06X} r1={:06X} r2={:06X} fp={:06X} sp={:06X} c={}",
            self.cpu.get_reg(0), self.cpu.get_reg(1), self.cpu.get_reg(2),
            self.cpu.get_reg(3), self.cpu.get_reg(4), self.cpu.c as u8);
    }
}

/// Parse an address from hex (0x...) or decimal
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

/// Disassemble one instruction at the given address.
/// Returns (text, instruction_size_in_bytes).
fn disassemble_at(cpu: &CpuState, pc: u32) -> (String, usize) {
    let byte0 = cpu.read_byte(pc);
    let decoded = DECODE_ROM[byte0 as usize];

    if decoded == 0xFFF {
        return (format!(".byte 0x{:02X}", byte0), 1);
    }

    let opcode = ((decoded >> 6) & 0x1F) as u8;
    let ra = ((decoded >> 3) & 0x07) as u8;
    let rb = (decoded & 0x07) as u8;

    let reg_name = |r: u8| -> &'static str {
        match r {
            0 => "r0", 1 => "r1", 2 => "r2", 3 => "fp",
            4 => "sp", 5 => "z", 6 => "iv", 7 => "ir",
            _ => "??",
        }
    };

    match opcode {
        // 1-byte register-register instructions
        0x00 => (format!("add  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x02 => (format!("and  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x06 => (format!("ceq  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x07 => (format!("cls  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x08 => (format!("clu  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x09 => (format!("jal  {},({})", reg_name(ra), reg_name(rb)), 1),
        0x0A => (format!("jmp  ({})", reg_name(ra)), 1),
        0x11 => (format!("mov  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x12 => (format!("mul  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x13 => (format!("or   {},{}", reg_name(ra), reg_name(rb)), 1),
        0x14 => (format!("pop  {}", reg_name(ra)), 1),
        0x15 => (format!("push {}", reg_name(ra)), 1),
        0x17 => (format!("shl  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x18 => (format!("sra  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x19 => (format!("srl  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x1A => (format!("sub  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x1D => (format!("sxt  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x1E => (format!("xor  {},{}", reg_name(ra), reg_name(rb)), 1),
        0x1F => (format!("zxt  {},{}", reg_name(ra), reg_name(rb)), 1),

        // 2-byte with signed immediate
        0x01 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("add  {},{}", reg_name(ra), dd), 2)
        }
        0x03 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            let target = (pc + 2).wrapping_add(CpuState::sign_extend_8(dd as u8));
            (format!("bra  0x{:06X}", target & 0xFFFFFF), 2)
        }
        0x04 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            let target = (pc + 2).wrapping_add(CpuState::sign_extend_8(dd as u8));
            (format!("brf  0x{:06X}", target & 0xFFFFFF), 2)
        }
        0x05 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            let target = (pc + 2).wrapping_add(CpuState::sign_extend_8(dd as u8));
            (format!("brt  0x{:06X}", target & 0xFFFFFF), 2)
        }
        0x0C => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("lb   {},{}({})", reg_name(ra), dd, reg_name(rb)), 2)
        }
        0x0D => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("lbu  {},{}({})", reg_name(ra), dd, reg_name(rb)), 2)
        }
        0x0E => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("lc   {},{}", reg_name(ra), dd), 2)
        }
        0x0F => {
            let dd = cpu.read_byte(pc + 1);
            (format!("lcu  {},{}", reg_name(ra), dd), 2)
        }
        0x10 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("lw   {},{}({})", reg_name(ra), dd, reg_name(rb)), 2)
        }
        0x16 => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("sb   {},{}({})", reg_name(ra), dd, reg_name(rb)), 2)
        }
        0x1C => {
            let dd = cpu.read_byte(pc + 1) as i8;
            (format!("sw   {},{}({})", reg_name(ra), dd, reg_name(rb)), 2)
        }

        // 4-byte instructions
        0x0B => {
            let imm = cpu.read_byte(pc + 1) as u32
                | ((cpu.read_byte(pc + 2) as u32) << 8)
                | ((cpu.read_byte(pc + 3) as u32) << 16);
            (format!("la   {},0x{:06X}", reg_name(ra), imm), 4)
        }
        0x1B => {
            let imm = cpu.read_byte(pc + 1) as u32
                | ((cpu.read_byte(pc + 2) as u32) << 8)
                | ((cpu.read_byte(pc + 3) as u32) << 16);
            (format!("sub  sp,0x{:06X}", imm), 4)
        }

        _ => (format!("??? op=0x{:02X}", opcode), 1),
    }
}
