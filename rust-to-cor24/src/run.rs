//! cor24-run: COR24 assembler and emulator CLI
//!
//! Usage:
//!   cor24-run --demo                              Run built-in LED demo
//!   cor24-run --demo --speed 50000 --time 10      Run at 50k IPS for 10 seconds
//!   cor24-run --run <file.s>                      Assemble and run
//!   cor24-run --assemble <in.s> <out.bin> <out.lst>  Assemble to binary + listing

use cor24_emulator::assembler::Assembler;
use cor24_emulator::emulator::EmulatorCore;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};

/// Default emulation speed (instructions per second)
const DEFAULT_SPEED: u64 = 100_000;

/// Default time limit in seconds
const DEFAULT_TIME_LIMIT: f64 = 10.0;

fn print_leds(leds: u8) {
    print!("\rLEDs: ");
    for i in (0..8).rev() {
        if (leds >> i) & 1 == 1 { print!("\x1b[91m●\x1b[0m"); }
        else { print!("○"); }
    }
    print!("  (0x{:02X})  ", leds);
    std::io::stdout().flush().ok();
}

/// Run emulator with timing, instruction limit, and queued UART input.
/// UART input bytes are fed one at a time after each batch, simulating
/// character-by-character typing at the emulated UART RX register.
fn run_with_timing(emu: &mut EmulatorCore, speed: u64, time_limit: f64, max_instructions: i64, uart_input: &[u8]) -> u64 {
    let start = Instant::now();
    let time_limit_duration = Duration::from_secs_f64(time_limit);

    let batch_size: u64 = if speed == 0 { 10000 } else { (speed / 100).max(1) };
    let batch_duration = if speed == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(batch_size as f64 / speed as f64)
    };

    let mut total_instructions: u64 = 0;
    let mut batch_start = Instant::now();
    let mut prev_led = emu.get_led();
    let mut prev_uart_len = 0usize;
    let mut uart_input_pos = 0usize;

    emu.resume();

    loop {
        if start.elapsed() >= time_limit_duration {
            break;
        }

        if max_instructions >= 0 && total_instructions >= max_instructions as u64 {
            break;
        }

        let this_batch = if max_instructions >= 0 {
            let remaining = (max_instructions as u64).saturating_sub(total_instructions);
            batch_size.min(remaining).max(1)
        } else {
            batch_size
        };

        let result = emu.run_batch(this_batch);
        total_instructions += result.instructions_run;

        // Check for LED changes
        let led = emu.get_led();
        if led != prev_led {
            print_leds(led);
            prev_led = led;
        }

        // Print any new UART output
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            let new_chars = &output[prev_uart_len..];
            for ch in new_chars.chars() {
                if ch == '\n' {
                    println!("[UART TX @ {}] '\\n'", total_instructions);
                } else {
                    println!("[UART TX @ {}] '{}'  (0x{:02X})", total_instructions, ch, ch as u8);
                }
            }
            prev_uart_len = output.len();
        }

        // Feed next UART input character when previous was consumed (FIFO drain)
        // Only send when RX ready bit (bit 0 of status register) is clear,
        // meaning the program has read the previous byte.
        if uart_input_pos < uart_input.len() {
            let uart_status = emu.read_byte(0xFF0101);
            if uart_status & 0x01 == 0 {
                let ch = uart_input[uart_input_pos];
                emu.send_uart_byte(ch);
                if ch == b'!' {
                    println!("[UART RX] '!'  (0x21) — halt signal");
                } else if ch == b'\n' {
                    println!("[UART RX] '\\n'");
                } else {
                    println!("[UART RX] '{}'  (0x{:02X})", ch as char, ch);
                }
                uart_input_pos += 1;
            }
        }

        if result.instructions_run == 0 {
            break; // halted or paused
        }

        if speed > 0 {
            let elapsed = batch_start.elapsed();
            if elapsed < batch_duration {
                thread::sleep(batch_duration - elapsed);
            }
            batch_start = Instant::now();
        }
    }

    total_instructions
}

/// Load assembled bytes into emulator at their correct addresses
fn load_assembled(emu: &mut EmulatorCore, result: &cor24_emulator::assembler::AssemblyResult) {
    for line in &result.lines {
        if !line.bytes.is_empty() {
            for (i, &b) in line.bytes.iter().enumerate() {
                emu.write_byte(line.address + i as u32, b);
            }
        }
    }
}

/// LED counter demo with spin loop delay
const DEMO_SOURCE: &str = r#"
; LED Counter Demo with Spin Loop Delay
; Counts 0-255 on LEDs, loops forever

        push    fp
        mov     fp, sp
        add     sp, -3

        la      r1, -65536
        lc      r0, 0
        sw      r0, 0(fp)

main_loop:
        lw      r0, 0(fp)
        sb      r0, 0(r1)

        la      r2, 15000
delay:
        lc      r0, 1
        sub     r2, r0
        brt     delay

        lw      r0, 0(fp)
        lc      r2, 1
        add     r0, r2
        sw      r0, 0(fp)

        bra     main_loop
"#;

struct CliArgs {
    command: String,
    speed: u64,
    time_limit: f64,
    max_instructions: i64,
    file: Option<String>,
    dump: bool,
    entry: Option<String>,           // entry point label
    uart_input: Vec<u8>,             // characters to send to UART RX
    trace: usize,                    // number of trace entries to dump (0 = off)
    step: bool,                      // step mode: print each instruction
    uart_never_ready: bool,          // UART TX never becomes ready (test polling)
    terminal: bool,                  // bridge stdin/stdout to UART
    echo: bool,                      // echo stdin to stdout in terminal mode
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = env::args().collect();
    let mut cli = CliArgs {
        command: String::new(),
        speed: DEFAULT_SPEED,
        time_limit: DEFAULT_TIME_LIMIT,
        max_instructions: -1,
        file: None,
        dump: false,
        entry: None,
        uart_input: Vec::new(),
        trace: 0,
        step: false,
        uart_never_ready: false,
        terminal: false,
        echo: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--demo" => cli.command = "demo".to_string(),
            "--run" => {
                cli.command = "run".to_string();
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    cli.file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--assemble" => {
                cli.command = "assemble".to_string();
            }
            "--speed" | "-s" => {
                if i + 1 < args.len() {
                    cli.speed = args[i + 1].parse().unwrap_or(DEFAULT_SPEED);
                    i += 1;
                }
            }
            "--time" | "-t" => {
                if i + 1 < args.len() {
                    cli.time_limit = args[i + 1].parse().unwrap_or(DEFAULT_TIME_LIMIT);
                    i += 1;
                }
            }
            "--dump" => {
                cli.dump = true;
            }
            "--max-instructions" | "-n" => {
                if i + 1 < args.len() {
                    cli.max_instructions = args[i + 1].parse().unwrap_or(-1);
                    i += 1;
                }
            }
            "--uart-input" | "-u" => {
                if i + 1 < args.len() {
                    // Parse escape sequences: \n, \x21, etc.
                    let s = &args[i + 1];
                    let mut bytes = Vec::new();
                    let mut chars = s.chars().peekable();
                    while let Some(ch) = chars.next() {
                        if ch == '\\' {
                            match chars.next() {
                                Some('n') => bytes.push(b'\n'),
                                Some('r') => bytes.push(b'\r'),
                                Some('\\') => bytes.push(b'\\'),
                                Some('x') => {
                                    let hi = chars.next().unwrap_or('0');
                                    let lo = chars.next().unwrap_or('0');
                                    let hex = format!("{}{}", hi, lo);
                                    bytes.push(u8::from_str_radix(&hex, 16).unwrap_or(0));
                                }
                                Some(c) => { bytes.push(b'\\'); bytes.push(c as u8); }
                                None => bytes.push(b'\\'),
                            }
                        } else {
                            bytes.push(ch as u8);
                        }
                    }
                    cli.uart_input = bytes;
                    i += 1;
                }
            }
            "--entry" | "-e" => {
                if i + 1 < args.len() {
                    cli.entry = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--trace" => {
                if i + 1 < args.len() {
                    cli.trace = args[i + 1].parse().unwrap_or(50);
                    i += 1;
                } else {
                    cli.trace = 50;
                }
            }
            "--step" => {
                cli.step = true;
            }
            "--uart-never-ready" => {
                cli.uart_never_ready = true;
            }
            "--terminal" => {
                cli.terminal = true;
            }
            "--echo" => {
                cli.echo = true;
            }
            _ => {
                if cli.command.is_empty() && !args[i].starts_with('-') {
                    cli.file = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    cli
}


/// Print one row of 16 bytes in hex + ASCII
fn print_hex_row(emu: &EmulatorCore, addr: u32) {
    print!("  {:06X}:", addr);
    for j in 0..16u32 {
        print!(" {:02X}", emu.read_byte(addr + j));
    }
    print!("  |");
    for j in 0..16u32 {
        let b = emu.read_byte(addr + j);
        if (0x20..=0x7E).contains(&b) {
            print!("{}", b as char);
        } else {
            print!(".");
        }
    }
    println!("|");
}

/// Check if a 16-byte row is all zero
fn row_is_zero(emu: &EmulatorCore, addr: u32) -> bool {
    for j in 0..16u32 {
        if emu.read_byte(addr + j) != 0 {
            return false;
        }
    }
    true
}

/// Dump a memory region, collapsing runs of zero rows.
/// Shows non-zero rows verbatim; consecutive zero rows are summarized.
fn dump_memory_region(emu: &EmulatorCore, start: u32, end: u32) {
    let mut addr = start & !0xF; // align to 16
    while addr <= end {
        if row_is_zero(emu, addr) {
            // Count consecutive zero rows
            let zero_start = addr;
            while addr <= end && row_is_zero(emu, addr) {
                addr += 16;
            }
            let zero_bytes = addr - zero_start;
            if zero_bytes <= 16 {
                // Single zero row — just print it
                print_hex_row(emu, zero_start);
            } else {
                println!("  {:06X}..{:06X}: {} bytes all zero", zero_start, addr - 1, zero_bytes);
            }
        } else {
            print_hex_row(emu, addr);
            addr += 16;
        }
    }
}

/// Print I/O state in a human-readable format
fn print_io_state(emu: &EmulatorCore) {
    let snap = emu.snapshot();
    println!("\n=== I/O FF0000-FFFFFF (64 KB, memory-mapped peripherals) ===");

    // LED/Switch at 0xFF0000
    // Note: read_byte(0xFF0000) returns switch state; LED state is separate
    let led = snap.led;
    let btn = snap.button;
    print!("  FF0000 LED:  0x{:02X}  [", led);
    for i in (0..8).rev() {
        if (led >> i) & 1 == 1 { print!("*"); } else { print!("."); }
    }
    print!("]  BTN S2: ");
    // button field: normally high (1=released), 0=pressed
    println!("{}", if btn & 1 == 0 { "PRESSED" } else { "released" });

    // Interrupt enable at 0xFF0010
    let ie = emu.read_byte(0xFF0010);
    println!("  FF0010 IntEn:  0x{:02X}  UART RX IRQ: {}", ie, if ie & 1 == 1 { "enabled" } else { "disabled" });

    // UART
    let uart_stat = emu.read_byte(0xFF0101);
    println!("  FF0100 UART:   status=0x{:02X}  RX ready: {}  CTS: {}  TX busy: {}",
             uart_stat,
             if uart_stat & 1 == 1 { "yes" } else { "no" },
             if uart_stat & 2 == 2 { "yes" } else { "no" },
             if uart_stat & 0x80 == 0x80 { "yes" } else { "no" });

    let uart_out = emu.get_uart_output();
    if !uart_out.is_empty() {
        let escaped: String = uart_out.chars().map(|c| {
            if c == '\n' { "\\n".to_string() }
            else if c == '\r' { "\\r".to_string() }
            else { c.to_string() }
        }).collect();
        println!("  UART TX log:   \"{}\"", escaped);
    }
}

/// Print register and full memory dump
///
/// COR24 24-bit address space:
///   000000-0FFFFF  SRAM (1 MB) — code at low addresses, data/globals above
///   100000-FDFFFF  Unmapped (~14 MB gap, reads 0, writes ignored)
///   FE0000-FEDDFF  Unmapped (below EBR)
///   FEE000-FEFFFF  EBR (8 KB embedded block RAM) — stack (SP init = FEEC00)
///   FF0000-FFFFFF  I/O (64 KB, 4 registers mapped, rest reads 0)
fn print_dump(emu: &EmulatorCore) {
    let snap = emu.snapshot();
    println!("\n=== Registers ===");
    println!("  PC:  0x{:06X}    C: {}", snap.pc, if snap.c { "1" } else { "0" });
    println!("  r0:  0x{:06X}  ({:8})", snap.regs[0], snap.regs[0]);
    println!("  r1:  0x{:06X}  ({:8})", snap.regs[1], snap.regs[1]);
    println!("  r2:  0x{:06X}  ({:8})", snap.regs[2], snap.regs[2]);
    println!("  fp:  0x{:06X}", snap.regs[3]);
    println!("  sp:  0x{:06X}", snap.regs[4]);
    println!("\n=== Emulator ===");
    println!("  Instructions: {}", snap.instructions);
    println!("  Halted: {}", snap.halted);

    // --- Region 1: SRAM (000000-0FFFFF) ---
    let sram = emu.sram();
    let sram_end = sram.iter().rposition(|&b| b != 0)
        .map(|pos| ((pos as u32) | 0xF) + 1)
        .unwrap_or(0);
    println!("\n=== SRAM 000000-0FFFFF (1 MB) ===");
    if sram_end > 0 {
        dump_memory_region(emu, 0x000000, sram_end - 1);
        if sram_end < 0x100000 {
            println!("  {:06X}..0FFFFF: {} bytes all zero",
                     sram_end, 0x100000 - sram_end);
        }
    } else {
        println!("  000000..0FFFFF: 1048576 bytes all zero");
    }

    // --- Region 2: Unmapped gap ---
    println!("\n=== Unmapped 100000-FEDDFF (14.9 MB, not installed) ===");

    // --- Region 3: EBR / Stack (FEE000-FEFFFF) ---
    println!("\n=== EBR FEE000-FEFFFF (8 KB, stack) ===");
    let ebr = emu.ebr();
    if ebr.iter().any(|&b| b != 0) {
        dump_memory_region(emu, 0xFEE000, 0xFEFFFF);
    } else {
        println!("  FEE000..FEFFFF: 8192 bytes all zero");
    }

    // --- Region 4: I/O (FF0000-FFFFFF) ---
    print_io_state(emu);
}

/// Run in step mode: execute one instruction at a time, printing each.
/// Stops on halt, max_instructions limit, or loop detection.
fn run_step_mode(emu: &mut EmulatorCore, max_instructions: i64, uart_input: &[u8]) {
    let mut uart_pos = 0usize;
    let mut prev_uart_len = 0usize;
    let max = if max_instructions < 0 { 10_000 } else { max_instructions as u64 };

    println!("{:>5} {:>8}  {:<24}  {}", "#", "PC", "Instruction", "Changes");
    println!("{}", "-".repeat(80));

    for n in 0..max {
        // Feed UART input when previous byte consumed (FIFO drain)
        if uart_pos < uart_input.len() {
            let uart_status = emu.read_byte(0xFF0101);
            if uart_status & 0x01 == 0 {
                let ch = uart_input[uart_pos];
                emu.send_uart_byte(ch);
                println!("  --- UART RX: 0x{:02X} ('{}') ---",
                    ch, if (0x20..=0x7E).contains(&ch) { ch as char } else { '.' });
                uart_pos += 1;
            }
        }

        let result = emu.step();

        // Print the trace entry for this instruction
        let trace = emu.trace();
        if let Some(entry) = trace.last_n(1).first() {
            println!("{}", entry);
        }

        // Print any new UART output
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            let new = &output[prev_uart_len..];
            for ch in new.chars() {
                if ch == '\n' {
                    println!("  >>> UART TX: '\\n'");
                } else {
                    println!("  >>> UART TX: '{}'  (0x{:02X})", ch, ch as u8);
                }
            }
            prev_uart_len = output.len();
        }

        if result.instructions_run == 0 {
            println!("\n--- Halted after {} instructions ---", n);
            break;
        }
    }

    let uart = emu.get_uart_output();
    if !uart.is_empty() {
        println!("\nUART output: {}", uart);
    }
    println!("\nExecuted {} instructions", emu.instructions_count());
    if emu.is_halted() {
        println!("CPU halted (self-branch detected)");
    }
}

// --- Terminal mode (raw termios) ---

/// RAII guard that restores terminal settings on drop.
struct TermiosGuard {
    fd: libc::c_int,
    original: libc::termios,
}

impl Drop for TermiosGuard {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.original); }
    }
}

/// Put stdin into raw mode (character-at-a-time, no echo, no signals).
/// Returns a guard that restores the original settings on drop.
fn set_raw_mode() -> Result<TermiosGuard, String> {
    unsafe {
        let fd = libc::STDIN_FILENO;
        let mut original: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut original) != 0 {
            return Err("tcgetattr failed".to_string());
        }
        let mut raw = original;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        raw.c_iflag &= !(libc::IXON | libc::ICRNL);
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 0;
        if libc::tcsetattr(fd, libc::TCSAFLUSH, &raw) != 0 {
            return Err("tcsetattr failed".to_string());
        }
        Ok(TermiosGuard { fd, original })
    }
}

/// Run the emulator in terminal mode: stdin→UART RX, UART TX→stdout.
fn run_terminal_mode(emu: &mut EmulatorCore, speed: u64, time_limit: f64, max_instructions: i64, echo: bool) -> u64 {
    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) } != 0;

    let _guard = if is_tty {
        match set_raw_mode() {
            Ok(g) => {
                // Also install a panic hook that restores terminal
                let orig = g.original;
                let prev_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &orig); }
                    prev_hook(info);
                }));
                Some(g)
            }
            Err(e) => {
                eprintln!("Warning: could not set raw mode: {}", e);
                None
            }
        }
    } else {
        None
    };

    if is_tty {
        // Use \r\n since we're in raw mode (no output processing)
        eprint!("[cor24-run terminal mode \u{2014} Ctrl-] to exit]\r\n");
    }

    let batch_size: u64 = if speed == 0 { 10_000 } else { (speed / 100).max(100) };
    let batch_duration = if speed == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(batch_size as f64 / speed as f64)
    };

    let time_limit_duration = if time_limit <= 0.0 {
        Duration::from_secs(3600) // 1 hour default for terminal mode
    } else {
        Duration::from_secs_f64(time_limit)
    };

    let start = Instant::now();
    let mut total_instructions: u64 = 0;
    let mut batch_start = Instant::now();
    let mut prev_uart_len = 0usize;
    let mut stdin_buf: VecDeque<u8> = VecDeque::new();
    let mut read_buf = [0u8; 256];
    let stdin_fd = libc::STDIN_FILENO;
    let mut stdout = std::io::stdout();
    let mut stdin_eof = false;

    emu.resume();

    loop {
        if start.elapsed() >= time_limit_duration {
            break;
        }
        if max_instructions >= 0 && total_instructions >= max_instructions as u64 {
            break;
        }

        let this_batch = if max_instructions >= 0 {
            let remaining = (max_instructions as u64).saturating_sub(total_instructions);
            batch_size.min(remaining).max(1)
        } else {
            batch_size
        };

        let result = emu.run_batch(this_batch);
        total_instructions += result.instructions_run;

        // TX: flush new UART output to stdout as raw bytes
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            let new_bytes = &output.as_bytes()[prev_uart_len..];
            if is_tty {
                // In raw mode, translate \n to \r\n for proper terminal display
                for &b in new_bytes {
                    if b == b'\n' {
                        let _ = stdout.write_all(b"\r\n");
                    } else {
                        let _ = stdout.write_all(&[b]);
                    }
                }
            } else {
                let _ = stdout.write_all(new_bytes);
            }
            let _ = stdout.flush();
            prev_uart_len = output.len();
        }

        // RX: non-blocking read from stdin
        if !stdin_eof {
            let n = unsafe {
                libc::read(stdin_fd, read_buf.as_mut_ptr() as *mut libc::c_void, read_buf.len())
            };
            if n > 0 {
                let mut did_echo = false;
                for &b in &read_buf[..n as usize] {
                    if b == 0x1D {
                        // Ctrl-] — exit
                        if is_tty {
                            eprint!("\r\n[cor24-run exited]\r\n");
                        }
                        return total_instructions;
                    }
                    if stdin_buf.len() < 4096 {
                        stdin_buf.push_back(b);
                    }
                    if echo {
                        match b {
                            b'\r' | b'\n' => { let _ = stdout.write_all(b"\r\n"); }
                            0x08 | 0x7F => { let _ = stdout.write_all(b"\x08 \x08"); }
                            0x20..=0x7E => { let _ = stdout.write_all(&[b]); }
                            _ => {} // don't echo control characters
                        }
                        did_echo = true;
                    }
                }
                if did_echo {
                    let _ = stdout.flush();
                }
            } else if n == 0 && !is_tty {
                // EOF on piped stdin
                stdin_eof = true;
            }
        }

        // Feed buffered input to UART when ready
        if !stdin_buf.is_empty() {
            let status = emu.read_byte(0xFF0101);
            if status & 0x01 == 0 {
                let ch = stdin_buf.pop_front().unwrap();
                emu.send_uart_byte(ch);
            }
        }

        if result.instructions_run == 0 {
            if is_tty {
                eprint!("\r\n[CPU halted]\r\n");
            } else {
                eprintln!("\n[CPU halted]");
            }
            break;
        }

        // Timing synchronization
        if speed > 0 {
            let elapsed = batch_start.elapsed();
            if elapsed < batch_duration {
                thread::sleep(batch_duration - elapsed);
            }
            batch_start = Instant::now();
        }
    }

    if is_tty && start.elapsed() >= time_limit_duration {
        eprint!("\r\n[time limit reached]\r\n");
    }

    total_instructions
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("cor24-run: COR24 assembler and emulator\n");
        println!("Usage:");
        println!("  cor24-run --demo [options]        Run built-in LED demo");
        println!("  cor24-run --run <file.s> [opts]   Assemble and run");
        println!("  cor24-run --assemble <in.s> <out.bin> <out.lst>");
        println!();
        println!("Options:");
        println!("  --speed, -s <ips>    Instructions per second (default: {})", DEFAULT_SPEED);
        println!("  --time, -t <secs>    Time limit in seconds (default: {})", DEFAULT_TIME_LIMIT);
        println!("  --max-instructions, -n <count>  Stop after N instructions (-1 = no limit, default)");
        println!("  --uart-input, -u <str>  Send characters to UART RX (supports \\n, \\x21)");
        println!("  --dump               Dump CPU state, I/O, and non-zero memory after halt");
        println!("  --trace <N>          Dump last N instructions on halt/timeout (default: 50)");
        println!("  --step               Print each instruction as it executes");
        println!("  --uart-never-ready   UART TX stays busy forever (test polling)");
        println!("  --entry, -e <label>  Set entry point to label address");
        println!("  --terminal           Bridge stdin/stdout to UART (interactive mode)");
        println!("  --echo               Local echo in terminal mode (for programs that don't echo)");
        println!();
        println!("Example:");
        println!("  cor24-run --demo --speed 100000 --time 10");
        println!("  cor24-run --run prog.s --dump --speed 0");
        println!("  cor24-run --run echo.s -u 'abc!' --speed 0 --dump");
        println!("  cor24-run --run repl.s --terminal --speed 0");
        return;
    }

    let cli = parse_args();

    match cli.command.as_str() {
        "demo" => {
            println!("=== COR24 LED Demo ===\n");
            println!("Binary counter 0-255 on LEDs with spin loop delay");
            println!("Speed: {} instructions/sec, Time limit: {}s\n", cli.speed, cli.time_limit);

            let mut asm = Assembler::new();
            let result = asm.assemble(DEMO_SOURCE);
            if !result.errors.is_empty() {
                eprintln!("Assembly error: {}", result.errors.join("\n"));
                return;
            }

            println!("Program listing:");
            for line in &result.lines {
                if !line.bytes.is_empty() {
                    let bytes: String = line.bytes.iter().map(|b| format!("{:02X} ", b)).collect();
                    println!("{:04X}: {:14} {}", line.address, bytes.trim(), line.source);
                }
            }
            println!();

            let mut emu = EmulatorCore::new();
            load_assembled(&mut emu, &result);

            println!("Running (Ctrl+C to stop)...\n");
            let instructions = run_with_timing(&mut emu, cli.speed, cli.time_limit, cli.max_instructions, &cli.uart_input);

            println!("\n\nExecuted {} instructions in {:.1}s", instructions, cli.time_limit);
            println!("Effective speed: {:.0} IPS", instructions as f64 / cli.time_limit);
            if cli.dump { print_dump(&emu); }
        }

        "run" => {
            let filename = match cli.file {
                Some(f) => f,
                None => {
                    eprintln!("Usage: cor24-run --run <file.s>");
                    return;
                }
            };

            let source = fs::read_to_string(&filename).expect("Cannot read file");
            let mut asm = Assembler::new();
            let result = asm.assemble(&source);
            if !result.errors.is_empty() {
                eprintln!("Assembly errors:");
                for err in &result.errors {
                    eprintln!("  {}", err);
                }
                return;
            }

            let byte_count: usize = result.lines.iter().map(|l| l.bytes.len()).sum();
            println!("Assembled {} bytes", byte_count);

            // Set entry point if specified
            let mut emu = EmulatorCore::new();
            if cli.uart_never_ready {
                emu.set_uart_never_ready(true);
            }
            load_assembled(&mut emu, &result);

            if let Some(entry_label) = &cli.entry {
                // Find label address in assembly result
                let mut found = false;
                for line in &result.lines {
                    let src = line.source.trim();
                    if src.ends_with(':') && src.trim_end_matches(':') == entry_label.as_str() {
                        emu.set_pc(line.address);
                        println!("Entry point: {} @ 0x{:06X}", entry_label, line.address);
                        found = true;
                        break;
                    }
                }
                if !found {
                    eprintln!("Warning: entry point '{}' not found, starting at 0x000000", entry_label);
                }
            }

            if cli.echo && !cli.terminal {
                eprintln!("Error: --echo requires --terminal");
                return;
            }

            if cli.terminal {
                // Validate incompatible flags
                if !cli.uart_input.is_empty() {
                    eprintln!("Error: --terminal and --uart-input are incompatible");
                    return;
                }
                if cli.step {
                    eprintln!("Error: --terminal and --step are incompatible");
                    return;
                }

                let speed = if cli.speed == DEFAULT_SPEED { 0 } else { cli.speed };
                let time_limit = if cli.time_limit == DEFAULT_TIME_LIMIT { 0.0 } else { cli.time_limit };

                let instructions = run_terminal_mode(&mut emu, speed, time_limit, cli.max_instructions, cli.echo);

                eprintln!("Executed {} instructions", instructions);
                if cli.trace > 0 {
                    print!("{}", emu.trace().format_last(cli.trace));
                }
                if cli.dump { print_dump(&emu); }
                return;
            }

            println!("Running (speed: {} IPS, time limit: {}s)...\n",
                     if cli.speed == 0 { "max".to_string() } else { cli.speed.to_string() },
                     cli.time_limit);

            if cli.step {
                // Step mode: execute one instruction at a time, printing each
                run_step_mode(&mut emu, cli.max_instructions, &cli.uart_input);
            } else {
                let instructions = run_with_timing(&mut emu, cli.speed, cli.time_limit, cli.max_instructions, &cli.uart_input);

                // Print UART output if any
                let uart = emu.get_uart_output();
                if !uart.is_empty() {
                    println!("\nUART output: {}", uart);
                }

                println!("\nExecuted {} instructions", instructions);
                if emu.is_halted() {
                    println!("CPU halted (self-branch detected)");
                }
            }
            if cli.trace > 0 {
                print!("{}", emu.trace().format_last(cli.trace));
            }
            if cli.dump { print_dump(&emu); }
        }

        "assemble" => {
            if args.len() < 5 {
                eprintln!("Usage: cor24-run --assemble <in.s> <out.bin> <out.lst>");
                return;
            }
            let source = fs::read_to_string(&args[2]).expect("Cannot read file");
            let mut asm = Assembler::new();
            let result = asm.assemble(&source);
            if !result.errors.is_empty() {
                eprintln!("Assembly error: {}", result.errors.join("\n"));
                return;
            }

            let machine_code: Vec<u8> = result.lines.iter()
                .flat_map(|line| line.bytes.iter().copied())
                .collect();

            fs::write(&args[3], &machine_code).expect("Cannot write .bin");
            let mut lst_file = fs::File::create(&args[4]).expect("Cannot write .lst");
            for line in &result.lines {
                if !line.bytes.is_empty() {
                    let bytes: String = line.bytes.iter().map(|b| format!("{:02X} ", b)).collect();
                    writeln!(lst_file, "{:04X}: {:14} {}", line.address, bytes.trim(), line.source).ok();
                } else if !line.source.is_empty() {
                    writeln!(lst_file, "                    {}", line.source).ok();
                }
            }
            println!("Wrote {} bytes to {}", machine_code.len(), args[3]);
            println!("Wrote listing to {}", args[4]);
        }

        _ => {
            eprintln!("Unknown command. Use --demo, --run, or --assemble");
        }
    }
}
