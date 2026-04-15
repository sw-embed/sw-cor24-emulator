//! cor24-emu: COR24 assembler and emulator CLI
//!
//! Usage:
//!   cor24-emu --demo                              Run built-in LED demo
//!   cor24-emu --demo --speed 50000 --time 10      Run at 50k IPS for 10 seconds
//!   cor24-emu --run <file.s>                      Assemble and run
//!   cor24-emu --assemble <in.s> <out.bin> <out.lst>  Assemble to binary + listing

use cor24_emulator::assembler::{Assembler, AssemblyResult};
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

const COPYRIGHT: &str = "Copyright (c) 2026 Michael A Wright";
const LICENSE: &str = "MIT";
const REPOSITORY: &str = "https://github.com/sw-embed/sw-cor24-emulator";

fn print_version() {
    println!(
        "cor24-emu {}\n{}\nLicense: {}\nRepository: {}\n\nBuild Information:\n  Host: {}\n  Commit: {}\n  Timestamp: {}",
        env!("CARGO_PKG_VERSION"),
        COPYRIGHT,
        LICENSE,
        REPOSITORY,
        env!("VERGEN_BUILD_HOST"),
        env!("VERGEN_GIT_SHA_SHORT"),
        env!("VERGEN_BUILD_TIMESTAMP"),
    );
}

fn print_short_help() {
    println!("cor24-emu: COR24 assembler and emulator\n");
    println!("Usage:");
    println!("  cor24-emu --demo [options]        Run built-in LED demo");
    println!("  cor24-emu --run <file.s> [opts]   Assemble and run");
    println!("  cor24-emu --load-binary <f>@<a> --entry <a>  Run pre-assembled binaries");
    println!("  cor24-emu --assemble <in.s> <out.bin> <out.lst>");
    println!();
    println!("Options:");
    println!("  -h                     Short help (this message)");
    println!("  --help                 Extended help with AI agent guidance");
    println!("  -V, --version          Version, copyright, license, build info");
    println!(
        "  --speed, -s <ips>      Instructions per second (default: {})",
        DEFAULT_SPEED
    );
    println!(
        "  --time, -t <secs>      Time limit in seconds (default: {})",
        DEFAULT_TIME_LIMIT
    );
    println!("  --max-instructions, -n <count>  Stop after N instructions (-1 = no limit)");
    println!("  --uart-input, -u <str> Send characters to UART RX (supports \\n, \\x21)");
    println!("  --uart-file <path>     Read file contents into UART RX buffer (appends 0x04 EOF)");
    println!("  --quiet, -q            UART TX as plain text on stdout; logs to stderr");
    println!("  --entry, -e <label|addr> Set entry point (label name or numeric address)");
    println!("  --dump                 Dump CPU state, I/O, and non-zero memory after halt");
    println!("  --dump-uart            Show UART transaction log (chronological IN/OUT)");
    println!("  --trace <N>            Dump last N instructions on halt/timeout (default: 50)");
    println!("  --step                 Print each instruction as it executes");
    println!("  --terminal             Bridge stdin/stdout to UART (interactive mode)");
    println!("  --echo                 Local echo in terminal mode (for programs that don't echo)");
    println!("  --load-binary <file>@<addr>  Load raw bytes into memory at address");
    println!("  --patch <addr>=<value> Write 24-bit value to memory (repeatable)");
    println!("  --base-addr <addr>     Base address for assembly (default: 0)");
    println!("  --stack-kilobytes <3|8>  EBR stack size (default: 3, max: 8)");
    println!("  --switch <on|off>      Set button S2 state (default: off/released)");
    println!("  --uart-never-ready     UART TX stays busy forever (test polling)");
    println!(
        "  --guard-jumps          Halt if PC leaves the code region (catches bad control flow)"
    );
    println!("  --code-end <addr>      Upper bound for --guard-jumps (default: program_end)");
    println!("  --canary <addr>[=val]  Halt if memory at addr changes (default magic: 0xDEADBE)");
    println!("  --watch-range <lo> <hi> Halt if any byte in [lo, hi] changes (repeatable)");
    println!();
    println!("Examples:");
    println!("  cor24-emu --demo --speed 100000 --time 10");
    println!("  cor24-emu --run prog.s --dump --speed 0");
    println!("  cor24-emu --run echo.s -u 'abc!' --speed 0 --dump --dump-uart");
    println!("  cor24-emu --run repl.s --terminal --echo --speed 0");
    println!("  cor24-emu --run pvm.s --load-binary hello.p24@0x010000 --terminal");
    println!(
        "  cor24-emu --load-binary pvm.bin@0 --load-binary hello.p24@0x010000 --entry 0 --terminal"
    );
    println!("  cor24-emu --load-binary pvm.bin@0 --patch 0x09D7=0x010000 --entry 0 --terminal");
}

fn print_long_help() {
    print_short_help();
    println!();
    println!("=== Extended Help ===");
    println!();
    println!("COR24 Architecture:");
    println!("  24-bit RISC CPU (C-Oriented RISC) designed for embedded systems education.");
    println!("  3 general-purpose registers (r0, r1, r2), frame pointer (fp), stack pointer (sp).");
    println!("  Variable-length instructions (1/2/4 bytes). 24-bit address space (16 MB).");
    println!();
    println!("Memory Map:");
    println!("  000000-0FFFFF  SRAM (1 MB) — code and data");
    println!("  FEE000-FEFFFF  EBR (8 KB) — stack (3 KB default, 8 KB with --stack-kilobytes 8)");
    println!("  FF0000-FFFFFF  I/O — LED/switch at FF0000, UART at FF0100-FF0101");
    println!();
    println!("Terminal Mode (--terminal):");
    println!("  Bridges stdin/stdout directly to the emulated UART for interactive programs");
    println!("  (REPLs, shells, monitors). Raw terminal mode: Ctrl-C sends 0x03 to UART,");
    println!("  Ctrl-] exits. Use --echo for programs that don't echo typed characters.");
    println!("  Defaults to max speed and 1-hour time limit.");
    println!(
        "  Pipe-aware: works with piped input (echo '(+ 1 2)' | cor24-emu --run repl.s --terminal)."
    );
    println!();
    println!("UART I/O Registers:");
    println!("  FF0100  Data: write to transmit, read to receive (auto-acknowledges RX)");
    println!("  FF0101  Status: bit 0 = RX ready, bit 1 = CTS, bit 7 = TX busy");
    println!();
    println!("AI Agent Guidance:");
    println!("  This tool assembles COR24 assembly (.s files) and runs them on an emulator.");
    println!("  Assembly syntax follows the reference as24 assembler: labels on their own line,");
    println!("  hex literals use FFh suffix (not 0xFF prefix), la for 24-bit immediates.");
    println!("  The --dump flag is invaluable for debugging — it shows registers, stack, SRAM,");
    println!("  and I/O state. Use --trace N to see the last N executed instructions.");
    println!("  For interactive programs, use --terminal (optionally with --echo).");
    println!("  Programs that need deep recursion should use --stack-kilobytes 8.");
    println!("  Use --load-binary <file>@<addr> to load guest binaries (p24, forth, etc)");
    println!("  into memory after the host program is assembled. Repeatable for multiple files.");
    println!("  Files with .p24 magic header (P24\\0) are auto-detected: the 18-byte header");
    println!("  is stripped and only the code+data body is loaded.");
    println!();
    println!("  Use --patch <addr>=<value> to write 24-bit values to memory after loading.");
    println!("  Useful for setting VM state (e.g., guest_code_base). Repeatable.");
    println!();
    println!("  Binary-only mode: use --load-binary + --entry <addr> without --run to skip");
    println!("  assembly entirely. Load pre-assembled COR24 binaries for instant startup:");
    println!("    cor24-emu --load-binary pvm.bin@0 --load-binary hello.p24@0x010000 \\");
    println!("              --patch 0x09D7=0x010000 --entry 0 --terminal");
}

fn print_leds(leds: u8) {
    print!("\rLEDs: ");
    for i in (0..8).rev() {
        if (leds >> i) & 1 == 0 {
            print!("\x1b[91m●\x1b[0m");
        }
        // active-low: 0=ON
        else {
            print!("○");
        }
    }
    print!("  (0x{:02X})  ", leds);
    std::io::stdout().flush().ok();
}

/// Guards that halt execution when bad control flow or memory writes occur.
struct GuardState {
    guard_jumps: bool,
    code_end: u32,
    canaries: Vec<(u32, u32)>,
    watch_ranges: Vec<(u32, u32, Vec<u8>)>,
}

impl GuardState {
    fn install(cli: &CliArgs, emu: &mut EmulatorCore) -> Self {
        for &(addr, value) in &cli.canaries {
            emu.write_byte(addr, (value & 0xFF) as u8);
            emu.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
            emu.write_byte(addr + 2, ((value >> 16) & 0xFF) as u8);
        }
        let watch_ranges = cli
            .watch_ranges
            .iter()
            .map(|&(lo, hi)| {
                let snap: Vec<u8> = (lo..=hi).map(|a| emu.read_byte(a)).collect();
                (lo, hi, snap)
            })
            .collect();
        let code_end = cli.code_end.unwrap_or_else(|| {
            let pe = emu.program_end();
            if pe == 0 { 0x100000 } else { pe }
        });
        Self {
            guard_jumps: cli.guard_jumps,
            code_end,
            canaries: cli.canaries.clone(),
            watch_ranges,
        }
    }

    fn active(&self) -> bool {
        self.guard_jumps || !self.canaries.is_empty() || !self.watch_ranges.is_empty()
    }

    /// Returns a diagnostic message if a guard fired.
    fn check(&self, emu: &EmulatorCore) -> Option<String> {
        if self.guard_jumps {
            let pc = emu.pc();
            if pc >= self.code_end {
                return Some(format!(
                    "[GUARD] PC=0x{:06X} outside code region [0, 0x{:06X})",
                    pc, self.code_end
                ));
            }
        }
        for &(addr, expected) in &self.canaries {
            let actual = emu.read_word(addr);
            if actual != expected {
                return Some(format!(
                    "[GUARD] canary @ 0x{:06X} modified: expected 0x{:06X}, got 0x{:06X}",
                    addr, expected, actual
                ));
            }
        }
        for (lo, hi, snap) in &self.watch_ranges {
            for (i, &orig) in snap.iter().enumerate() {
                let addr = lo + i as u32;
                if addr > *hi {
                    break;
                }
                if emu.read_byte(addr) != orig {
                    return Some(format!(
                        "[GUARD] watch-range write @ 0x{:06X} in [0x{:06X}, 0x{:06X}]",
                        addr, lo, hi
                    ));
                }
            }
        }
        None
    }
}

/// Run emulator with timing, instruction limit, and queued UART input.
/// UART input bytes are fed one at a time after each batch, simulating
/// character-by-character typing at the emulated UART RX register.
fn run_with_timing(
    emu: &mut EmulatorCore,
    speed: u64,
    time_limit: f64,
    max_instructions: i64,
    uart_input: &[u8],
    quiet: bool,
    guard: &GuardState,
) -> u64 {
    let start = Instant::now();
    let time_limit_duration = Duration::from_secs_f64(time_limit);

    let batch_size: u64 = if guard.active() {
        // Smaller batches when guards are active so diagnostics fire close
        // to the offending instruction.
        if speed == 0 {
            256
        } else {
            (speed / 100).clamp(1, 256)
        }
    } else if speed == 0 {
        10000
    } else {
        (speed / 100).max(1)
    };
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
            if !quiet {
                print_leds(led);
            }
            prev_led = led;
        }

        // Print any new UART output
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            let new_chars = &output[prev_uart_len..];
            if quiet {
                let mut stdout = std::io::stdout();
                let _ = stdout.write_all(new_chars.as_bytes());
                let _ = stdout.flush();
            } else {
                for ch in new_chars.chars() {
                    if ch == '\n' {
                        println!("[UART TX @ {}] '\\n'", total_instructions);
                    } else {
                        println!(
                            "[UART TX @ {}] '{}'  (0x{:02X})",
                            total_instructions, ch, ch as u8
                        );
                    }
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
                if !quiet {
                    if ch == b'!' {
                        println!("[UART RX] '!'  (0x21) — halt signal");
                    } else if ch == b'\n' {
                        println!("[UART RX] '\\n'");
                    } else {
                        println!("[UART RX] '{}'  (0x{:02X})", ch as char, ch);
                    }
                }
                uart_input_pos += 1;
            }
        }

        if let Some(msg) = guard.check(emu) {
            eprintln!("\n{}", msg);
            break;
        }

        match result.reason {
            cor24_emulator::emulator::StopReason::StackOverflow(sp) => {
                eprintln!("\nStack overflow: SP=0x{:06X} below stack base", sp);
                break;
            }
            cor24_emulator::emulator::StopReason::StackUnderflow(sp) => {
                eprintln!("\nStack underflow: SP=0x{:06X} above stack top", sp);
                break;
            }
            _ if result.instructions_run == 0 => break, // halted or paused
            _ => {}
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
fn load_assembled(emu: &mut EmulatorCore, result: &AssemblyResult) {
    let mut end: u32 = 0;
    for line in &result.lines {
        if !line.bytes.is_empty() {
            for (i, &b) in line.bytes.iter().enumerate() {
                emu.write_byte(line.address + i as u32, b);
            }
            let line_end = line.address + line.bytes.len() as u32;
            if line_end > end {
                end = line_end;
            }
        }
    }
    if end > 0 {
        emu.load_program_extent(end);
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
    dump_uart: bool,
    entry: Option<String>,
    uart_input: Vec<u8>,
    trace: usize,
    step: bool,
    uart_never_ready: bool,
    terminal: bool,
    echo: bool,
    stack_kb: u32,
    load_binaries: Vec<(String, u32)>,
    patches: Vec<(u32, u32)>,
    base_addr: u32,
    switch_pressed: bool,
    quiet: bool,
    guard_jumps: bool,
    code_end: Option<u32>,
    canaries: Vec<(u32, u32)>,
    watch_ranges: Vec<(u32, u32)>,
}

/// Parse a numeric address string: 0x prefix, h suffix, or decimal.
fn parse_numeric_addr(s: &str) -> Option<u32> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).ok()
    } else if s.ends_with('h') || s.ends_with('H') {
        u32::from_str_radix(&s[..s.len() - 1], 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
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
        dump_uart: false,
        entry: None,
        uart_input: Vec::new(),
        trace: 0,
        step: false,
        uart_never_ready: false,
        terminal: false,
        echo: false,
        stack_kb: 3,
        load_binaries: Vec::new(),
        patches: Vec::new(),
        base_addr: 0,
        switch_pressed: false,
        quiet: false,
        guard_jumps: false,
        code_end: None,
        canaries: Vec::new(),
        watch_ranges: Vec::new(),
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
            "--dump-uart" => {
                cli.dump_uart = true;
            }
            "--max-instructions" | "-n" => {
                if i + 1 < args.len() {
                    cli.max_instructions = args[i + 1].parse().unwrap_or(-1);
                    i += 1;
                }
            }
            "--uart-input" | "-u" => {
                if i + 1 < args.len() {
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
                                Some(c) => {
                                    bytes.push(b'\\');
                                    bytes.push(c as u8);
                                }
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
            "--uart-file" => {
                if i + 1 < args.len() {
                    let path = &args[i + 1];
                    match fs::read(path) {
                        Ok(mut bytes) => {
                            bytes.push(0x04);
                            cli.uart_input = bytes;
                        }
                        Err(e) => {
                            eprintln!("Error: cannot read --uart-file '{}': {}", path, e);
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--quiet" | "-q" => {
                cli.quiet = true;
            }
            "--guard-jumps" => {
                cli.guard_jumps = true;
            }
            "--code-end" => {
                if i + 1 < args.len() {
                    match parse_numeric_addr(args[i + 1].trim()) {
                        Some(a) => cli.code_end = Some(a),
                        None => {
                            eprintln!("Error: invalid --code-end '{}'", args[i + 1]);
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--canary" => {
                if i + 1 < args.len() {
                    let spec = &args[i + 1];
                    let (addr_str, val_str) = match spec.split_once('=') {
                        Some((a, v)) => (a, v),
                        None => (spec.as_str(), "0xDEADBE"),
                    };
                    match (
                        parse_numeric_addr(addr_str.trim()),
                        parse_numeric_addr(val_str.trim()),
                    ) {
                        (Some(a), Some(v)) => cli.canaries.push((a, v)),
                        _ => {
                            eprintln!(
                                "Error: invalid --canary '{}' (expected <addr>[=<value>])",
                                spec
                            );
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--watch-range" => {
                if i + 2 < args.len() {
                    match (
                        parse_numeric_addr(args[i + 1].trim()),
                        parse_numeric_addr(args[i + 2].trim()),
                    ) {
                        (Some(lo), Some(hi)) if lo <= hi => cli.watch_ranges.push((lo, hi)),
                        _ => {
                            eprintln!(
                                "Error: invalid --watch-range '{} {}' (expected <lo> <hi>)",
                                args[i + 1],
                                args[i + 2]
                            );
                            std::process::exit(1);
                        }
                    }
                    i += 2;
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
            "--switch" => {
                if i + 1 < args.len() {
                    match args[i + 1].to_lowercase().as_str() {
                        "on" | "pressed" | "1" => cli.switch_pressed = true,
                        "off" | "released" | "0" => cli.switch_pressed = false,
                        _ => {
                            eprintln!("Error: --switch must be on or off");
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
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
            "--stack-kilobytes" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u32>() {
                        Ok(3) => cli.stack_kb = 3,
                        Ok(8) => cli.stack_kb = 8,
                        _ => {
                            eprintln!("Error: --stack-kilobytes must be 3 or 8");
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--load-binary" => {
                if i + 1 < args.len() {
                    let spec = &args[i + 1];
                    match spec.rsplit_once('@') {
                        Some((file, addr_str)) => match parse_numeric_addr(addr_str.trim()) {
                            Some(a) => cli.load_binaries.push((file.to_string(), a)),
                            None => {
                                eprintln!(
                                    "Error: invalid address in --load-binary '{}' (expected <file>@<addr>)",
                                    spec
                                );
                                std::process::exit(1);
                            }
                        },
                        None => {
                            eprintln!(
                                "Error: --load-binary requires <file>@<addr> format (e.g., hello.p24@0x010000)"
                            );
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--base-addr" => {
                if i + 1 < args.len() {
                    match parse_numeric_addr(args[i + 1].trim()) {
                        Some(a) => cli.base_addr = a,
                        None => {
                            eprintln!(
                                "Error: invalid --base-addr '{}' (expected address, e.g., 0x010000)",
                                args[i + 1]
                            );
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
            }
            "--patch" => {
                if i + 1 < args.len() {
                    let spec = &args[i + 1];
                    match spec.split_once('=') {
                        Some((addr_str, val_str)) => {
                            let addr = parse_numeric_addr(addr_str.trim());
                            let val = parse_numeric_addr(val_str.trim());
                            match (addr, val) {
                                (Some(a), Some(v)) => cli.patches.push((a, v)),
                                _ => {
                                    eprintln!(
                                        "Error: invalid --patch '{}' (expected <addr>=<value>, e.g., 0x09D7=0x010000)",
                                        spec
                                    );
                                    std::process::exit(1);
                                }
                            }
                        }
                        None => {
                            eprintln!(
                                "Error: --patch requires <addr>=<value> format (e.g., 0x09D7=0x010000)"
                            );
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                }
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
fn dump_memory_region(emu: &EmulatorCore, start: u32, end: u32) {
    let mut addr = start & !0xF; // align to 16
    while addr <= end {
        if row_is_zero(emu, addr) {
            let zero_start = addr;
            while addr <= end && row_is_zero(emu, addr) {
                addr += 16;
            }
            let zero_bytes = addr - zero_start;
            if zero_bytes <= 16 {
                print_hex_row(emu, zero_start);
            } else {
                println!(
                    "  {:06X}..{:06X}: {} bytes all zero",
                    zero_start,
                    addr - 1,
                    zero_bytes
                );
            }
        } else {
            print_hex_row(emu, addr);
            addr += 16;
        }
    }
}

/// Print I/O state in a human-readable format
fn print_io_state(emu: &EmulatorCore, dump_uart: bool) {
    let snap = emu.snapshot();
    println!("\n=== I/O FF0000-FFFFFF (64 KB, memory-mapped peripherals) ===");

    let led = snap.led;
    let switch = snap.button;
    print!("  LED D2:  0x{:02X}  ", led);
    if led & 1 == 0 {
        print!("ON (active-low)");
    } else {
        print!("off");
    }
    println!();
    print!("  BTN S2:  0x{:02X}  ", switch);
    println!(
        "{}",
        if switch & 1 == 0 {
            "PRESSED (active-low)"
        } else {
            "released"
        }
    );

    let ie = emu.read_byte(0xFF0010);
    println!(
        "  FF0010 IntEn:  0x{:02X}  UART RX IRQ: {}",
        ie,
        if ie & 1 == 1 { "enabled" } else { "disabled" }
    );

    let uart_stat = emu.read_byte(0xFF0101);
    println!(
        "  FF0100 UART:   status=0x{:02X}  RX ready: {}  CTS: {}  TX busy: {}",
        uart_stat,
        if uart_stat & 1 == 1 { "yes" } else { "no" },
        if uart_stat & 2 == 2 { "yes" } else { "no" },
        if uart_stat & 0x80 == 0x80 {
            "yes"
        } else {
            "no"
        }
    );

    let uart_out = emu.get_uart_output();
    if !uart_out.is_empty() {
        let escaped: String = uart_out
            .chars()
            .map(|c| {
                if c == '\n' {
                    "\\n".to_string()
                } else if c == '\r' {
                    "\\r".to_string()
                } else {
                    c.to_string()
                }
            })
            .collect();
        println!("  UART TX log:   \"{}\"", escaped);
    }

    if dump_uart {
        let log = emu.format_uart_log();
        if !log.is_empty() {
            let entry_count = emu.uart_log().entries().len();
            println!("  --- UART Transaction Log ({} entries) ---", entry_count);
            print!("{}", log);
        }
    }
}

/// Print register and full memory dump
fn print_dump(emu: &EmulatorCore, dump_uart: bool) {
    let snap = emu.snapshot();
    println!("\n=== Registers ===");
    println!(
        "  PC:  0x{:06X}    C: {}",
        snap.pc,
        if snap.c { "1" } else { "0" }
    );
    println!("  r0:  0x{:06X}  ({:8})", snap.regs[0], snap.regs[0]);
    println!("  r1:  0x{:06X}  ({:8})", snap.regs[1], snap.regs[1]);
    println!("  r2:  0x{:06X}  ({:8})", snap.regs[2], snap.regs[2]);
    println!("  fp:  0x{:06X}", snap.regs[3]);
    println!("  sp:  0x{:06X}", snap.regs[4]);
    println!("\n=== Emulator ===");
    println!("  Instructions: {}", snap.instructions);
    println!("  Halted: {}", snap.halted);

    // --- SRAM ---
    let sram = emu.sram();
    let sram_end = sram
        .iter()
        .rposition(|&b| b != 0)
        .map(|pos| ((pos as u32) | 0xF) + 1)
        .unwrap_or(0);
    println!("\n=== SRAM 000000-0FFFFF (1 MB) ===");
    if sram_end > 0 {
        dump_memory_region(emu, 0x000000, sram_end - 1);
        if sram_end < 0x100000 {
            println!(
                "  {:06X}..0FFFFF: {} bytes all zero",
                sram_end,
                0x100000 - sram_end
            );
        }
    } else {
        println!("  000000..0FFFFF: 1048576 bytes all zero");
    }

    // --- Unmapped ---
    println!("\n=== Unmapped 100000-FEDDFF (14.9 MB, not installed) ===");

    // --- EBR ---
    println!("\n=== EBR FEE000-FEFFFF (8 KB, stack) ===");
    let ebr = emu.ebr();
    if ebr.iter().any(|&b| b != 0) {
        dump_memory_region(emu, 0xFEE000, 0xFEFFFF);
    } else {
        println!("  FEE000..FEFFFF: 8192 bytes all zero");
    }

    // --- I/O ---
    print_io_state(emu, dump_uart);
}

/// Run in step mode: execute one instruction at a time, printing each.
fn run_step_mode(emu: &mut EmulatorCore, max_instructions: i64, uart_input: &[u8]) {
    let mut uart_pos = 0usize;
    let mut prev_uart_len = 0usize;
    let max = if max_instructions < 0 {
        10_000
    } else {
        max_instructions as u64
    };

    println!("{:>5} {:>8}  {:<24}  Changes", "#", "PC", "Instruction");
    println!("{}", "-".repeat(80));

    for n in 0..max {
        if uart_pos < uart_input.len() {
            let uart_status = emu.read_byte(0xFF0101);
            if uart_status & 0x01 == 0 {
                let ch = uart_input[uart_pos];
                emu.send_uart_byte(ch);
                println!(
                    "  --- UART RX: 0x{:02X} ('{}') ---",
                    ch,
                    if (0x20..=0x7E).contains(&ch) {
                        ch as char
                    } else {
                        '.'
                    }
                );
                uart_pos += 1;
            }
        }

        let result = emu.step();

        let trace = emu.trace();
        if let Some(entry) = trace.last_n(1).first() {
            println!("{}", entry);
        }

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
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.original);
        }
    }
}

/// Put stdin into raw mode.
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

/// Run the emulator in terminal mode: stdin->UART RX, UART TX->stdout.
fn run_terminal_mode(
    emu: &mut EmulatorCore,
    speed: u64,
    time_limit: f64,
    max_instructions: i64,
    echo: bool,
    preload: &[u8],
    guard: &GuardState,
) -> u64 {
    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) } != 0;

    let _guard = if is_tty {
        match set_raw_mode() {
            Ok(g) => {
                let orig = g.original;
                let prev_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    unsafe {
                        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &orig);
                    }
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
        eprint!("[cor24-emu terminal mode \u{2014} Ctrl-] to exit]\r\n");
    }

    let batch_size: u64 = if guard.active() {
        if speed == 0 {
            256
        } else {
            (speed / 100).clamp(100, 256)
        }
    } else if speed == 0 {
        10_000
    } else {
        (speed / 100).max(100)
    };
    let batch_duration = if speed == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(batch_size as f64 / speed as f64)
    };

    let time_limit_duration = if time_limit <= 0.0 {
        Duration::from_secs(3600)
    } else {
        Duration::from_secs_f64(time_limit)
    };

    let start = Instant::now();
    let mut total_instructions: u64 = 0;
    let mut batch_start = Instant::now();
    let mut prev_uart_len = 0usize;
    let mut stdin_buf: VecDeque<u8> = preload.iter().copied().collect();
    let mut read_buf = [0u8; 256];
    let stdin_fd = libc::STDIN_FILENO;
    let mut stdout = std::io::stdout();
    let mut stdin_eof = false;

    // For piped stdin, pre-buffer all input before starting emulation.
    // This avoids the blocking read() stalling the emulation loop and
    // ensures all piped bytes are available (fixes GitHub issue #2).
    if !is_tty {
        use std::io::Read;
        let mut all_input = Vec::new();
        if std::io::stdin().read_to_end(&mut all_input).is_ok() {
            stdin_buf.extend(all_input.iter());
            stdin_eof = true;
        }
    }

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

        // TX: flush new UART output to stdout
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            let new_bytes = &output.as_bytes()[prev_uart_len..];
            if is_tty {
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
                libc::read(
                    stdin_fd,
                    read_buf.as_mut_ptr() as *mut libc::c_void,
                    read_buf.len(),
                )
            };
            if n > 0 {
                let mut did_echo = false;
                for &b in &read_buf[..n as usize] {
                    if b == 0x1D {
                        // Ctrl-]
                        if is_tty {
                            eprint!("\r\n[cor24-emu exited]\r\n");
                        }
                        return total_instructions;
                    }
                    stdin_buf.push_back(b);
                    if echo {
                        match b {
                            b'\r' | b'\n' => {
                                let _ = stdout.write_all(b"\r\n");
                            }
                            0x08 | 0x7F => {
                                let _ = stdout.write_all(b"\x08 \x08");
                            }
                            0x20..=0x7E => {
                                let _ = stdout.write_all(&[b]);
                            }
                            _ => {}
                        }
                        did_echo = true;
                    }
                }
                if did_echo {
                    let _ = stdout.flush();
                }
            } else if n == 0 && !is_tty {
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

        if let Some(msg) = guard.check(emu) {
            if is_tty {
                eprint!("\r\n{}\r\n", msg);
            } else {
                eprintln!("\n{}", msg);
            }
            break;
        }

        match result.reason {
            cor24_emulator::emulator::StopReason::StackOverflow(sp)
            | cor24_emulator::emulator::StopReason::StackUnderflow(sp) => {
                let kind = if matches!(
                    result.reason,
                    cor24_emulator::emulator::StopReason::StackOverflow(_)
                ) {
                    "overflow"
                } else {
                    "underflow"
                };
                if is_tty {
                    eprint!("\r\n[Stack {}: SP=0x{:06X}]\r\n", kind, sp);
                } else {
                    eprintln!("\n[Stack {}: SP=0x{:06X}]", kind, sp);
                }
                break;
            }
            _ if result.instructions_run == 0 => {
                if is_tty {
                    eprint!("\r\n[CPU halted]\r\n");
                } else {
                    eprintln!("\n[CPU halted]");
                }
                break;
            }
            _ => {}
        }

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

/// .p24 magic bytes: "P24\0"
const P24_MAGIC: [u8; 4] = [0x50, 0x32, 0x34, 0x00];
const P24_HEADER_SIZE: usize = 18;

/// Load binary files and apply memory patches.
/// Auto-detects .p24 files by magic header and strips the 18-byte header.
fn load_binaries_and_patches(
    emu: &mut EmulatorCore,
    binaries: &[(String, u32)],
    patches: &[(u32, u32)],
) {
    for (file_path, addr) in binaries {
        let data = fs::read(file_path).unwrap_or_else(|e| {
            eprintln!("Error: cannot read binary file '{}': {}", file_path, e);
            std::process::exit(1);
        });

        let (body, stripped) = if data.len() >= P24_HEADER_SIZE && data[..4] == P24_MAGIC {
            (&data[P24_HEADER_SIZE..], true)
        } else {
            (data.as_slice(), false)
        };

        for (i, &b) in body.iter().enumerate() {
            emu.write_byte(addr + i as u32, b);
        }
        emu.load_program_extent(addr + body.len() as u32);
        if stripped {
            println!(
                "Loaded {} bytes from '{}' at 0x{:06X} (stripped {} byte .p24 header)",
                body.len(),
                file_path,
                addr,
                P24_HEADER_SIZE
            );
        } else {
            println!(
                "Loaded {} bytes from '{}' at 0x{:06X}",
                body.len(),
                file_path,
                addr
            );
        }
    }

    for &(addr, value) in patches {
        emu.write_byte(addr, (value & 0xFF) as u8);
        emu.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
        emu.write_byte(addr + 2, ((value >> 16) & 0xFF) as u8);
        println!("Patched 0x{:06X} = 0x{:06X}", addr, value);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle -h, --help, -V, --version before parsing other args
    if args.len() < 2 || args.contains(&"-h".to_string()) {
        print_short_help();
        return;
    }
    if args.contains(&"--help".to_string()) {
        print_long_help();
        return;
    }
    if args.contains(&"-V".to_string()) || args.contains(&"--version".to_string()) {
        print_version();
        return;
    }

    let mut cli = parse_args();

    // Binary-only mode: --load-binary without --run
    if cli.command.is_empty() && !cli.load_binaries.is_empty() {
        cli.command = "binary".to_string();
    }

    match cli.command.as_str() {
        "demo" => {
            println!("=== COR24 LED Demo ===\n");
            println!("Binary counter 0-255 on LEDs with spin loop delay");
            println!(
                "Speed: {} instructions/sec, Time limit: {}s\n",
                cli.speed, cli.time_limit
            );

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
            let guard = GuardState::install(&cli, &mut emu);
            let instructions = run_with_timing(
                &mut emu,
                cli.speed,
                cli.time_limit,
                cli.max_instructions,
                &cli.uart_input,
                cli.quiet,
                &guard,
            );

            println!(
                "\n\nExecuted {} instructions in {:.1}s",
                instructions, cli.time_limit
            );
            println!(
                "Effective speed: {:.0} IPS",
                instructions as f64 / cli.time_limit
            );
            if cli.dump {
                print_dump(&emu, cli.dump_uart);
            }
        }

        "run" => {
            let filename = match cli.file.clone() {
                Some(f) => f,
                None => {
                    eprintln!("Usage: cor24-emu --run <file.s>");
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
            if cli.quiet {
                eprintln!("Assembled {} bytes", byte_count);
            } else {
                println!("Assembled {} bytes", byte_count);
            }

            let mut emu = EmulatorCore::new();
            if cli.uart_never_ready {
                emu.set_uart_never_ready(true);
            }
            if cli.switch_pressed {
                emu.set_button_pressed(true);
            }
            if cli.stack_kb == 8 {
                emu.set_reg(4, 0xFF0000);
                emu.set_stack_bounds(cor24_emulator::cpu::state::EBR_BASE, 0xFF0000);
            }
            load_assembled(&mut emu, &result);

            load_binaries_and_patches(&mut emu, &cli.load_binaries, &cli.patches);

            if let Some(entry_str) = &cli.entry {
                if let Some(addr) = parse_numeric_addr(entry_str) {
                    emu.set_pc(addr);
                    println!("Entry point: 0x{:06X}", addr);
                } else {
                    let mut found = false;
                    for line in &result.lines {
                        let src = line.source.trim();
                        if src.ends_with(':') && src.trim_end_matches(':') == entry_str.as_str() {
                            emu.set_pc(line.address);
                            println!("Entry point: {} @ 0x{:06X}", entry_str, line.address);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        eprintln!(
                            "Warning: entry point '{}' not found, starting at 0x000000",
                            entry_str
                        );
                    }
                }
            }

            if cli.echo && !cli.terminal {
                eprintln!("Error: --echo requires --terminal");
                return;
            }

            if cli.terminal {
                if cli.step {
                    eprintln!("Error: --terminal and --step are incompatible");
                    return;
                }

                let speed = if cli.speed == DEFAULT_SPEED {
                    0
                } else {
                    cli.speed
                };
                let time_limit = if cli.time_limit == DEFAULT_TIME_LIMIT {
                    0.0
                } else {
                    cli.time_limit
                };

                let guard = GuardState::install(&cli, &mut emu);
                let instructions = run_terminal_mode(
                    &mut emu,
                    speed,
                    time_limit,
                    cli.max_instructions,
                    cli.echo,
                    &cli.uart_input,
                    &guard,
                );

                eprintln!("Executed {} instructions", instructions);
                if cli.trace > 0 {
                    print!("{}", emu.trace().format_last(cli.trace));
                }
                if cli.dump {
                    print_dump(&emu, cli.dump_uart);
                }
                return;
            }

            let run_msg = format!(
                "Running (speed: {} IPS, time limit: {}s)...\n",
                if cli.speed == 0 {
                    "max".to_string()
                } else {
                    cli.speed.to_string()
                },
                cli.time_limit
            );
            if cli.quiet {
                eprintln!("{}", run_msg);
            } else {
                println!("{}", run_msg);
            }

            if cli.step {
                run_step_mode(&mut emu, cli.max_instructions, &cli.uart_input);
            } else {
                let guard = GuardState::install(&cli, &mut emu);
                let instructions = run_with_timing(
                    &mut emu,
                    cli.speed,
                    cli.time_limit,
                    cli.max_instructions,
                    &cli.uart_input,
                    cli.quiet,
                    &guard,
                );

                if !cli.quiet {
                    let uart = emu.get_uart_output();
                    if !uart.is_empty() {
                        println!("\nUART output: {}", uart);
                    }
                    println!("\nExecuted {} instructions", instructions);
                    if emu.is_halted() {
                        println!("CPU halted (self-branch detected)");
                    }
                } else {
                    eprintln!("\nExecuted {} instructions", instructions);
                    if emu.is_halted() {
                        eprintln!("CPU halted (self-branch detected)");
                    }
                }
            }
            if cli.trace > 0 {
                print!("{}", emu.trace().format_last(cli.trace));
            }
            if cli.dump {
                print_dump(&emu, cli.dump_uart);
            }
        }

        "assemble" => {
            if args.len() < 5 {
                eprintln!("Usage: cor24-emu --assemble <in.s> <out.bin> <out.lst>");
                return;
            }
            let source = fs::read_to_string(&args[2]).expect("Cannot read file");
            let mut asm = Assembler::new();
            let result = asm.assemble_at(&source, cli.base_addr);
            if !result.errors.is_empty() {
                eprintln!("Assembly error: {}", result.errors.join("\n"));
                return;
            }

            let machine_code: Vec<u8> = result
                .lines
                .iter()
                .flat_map(|line| line.bytes.iter().copied())
                .collect();

            fs::write(&args[3], &machine_code).expect("Cannot write .bin");
            let mut lst_file = fs::File::create(&args[4]).expect("Cannot write .lst");
            for line in &result.lines {
                if !line.bytes.is_empty() {
                    let bytes: String = line.bytes.iter().map(|b| format!("{:02X} ", b)).collect();
                    writeln!(
                        lst_file,
                        "{:04X}: {:14} {}",
                        line.address,
                        bytes.trim(),
                        line.source
                    )
                    .ok();
                } else if !line.source.is_empty() {
                    writeln!(lst_file, "                    {}", line.source).ok();
                }
            }
            if cli.base_addr != 0 {
                println!(
                    "Assembled {} bytes at base 0x{:06X} to {}",
                    machine_code.len(),
                    cli.base_addr,
                    args[3]
                );
            } else {
                println!("Assembled {} bytes to {}", machine_code.len(), args[3]);
            }
            println!("Wrote listing to {}", args[4]);
        }

        "binary" => {
            let mut emu = EmulatorCore::new();
            if cli.uart_never_ready {
                emu.set_uart_never_ready(true);
            }
            if cli.switch_pressed {
                emu.set_button_pressed(true);
            }
            if cli.stack_kb == 8 {
                emu.set_reg(4, 0xFF0000);
                emu.set_stack_bounds(cor24_emulator::cpu::state::EBR_BASE, 0xFF0000);
            }

            load_binaries_and_patches(&mut emu, &cli.load_binaries, &cli.patches);

            if let Some(entry_str) = &cli.entry {
                match parse_numeric_addr(entry_str) {
                    Some(addr) => {
                        emu.set_pc(addr);
                        println!("Entry point: 0x{:06X}", addr);
                    }
                    None => {
                        eprintln!(
                            "Error: --entry must be a numeric address in binary mode (e.g., 0x000000)"
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                println!("Entry point: 0x000000 (default)");
            }

            if cli.echo && !cli.terminal {
                eprintln!("Error: --echo requires --terminal");
                return;
            }

            if cli.terminal {
                if cli.step {
                    eprintln!("Error: --terminal and --step are incompatible");
                    return;
                }

                let speed = if cli.speed == DEFAULT_SPEED {
                    0
                } else {
                    cli.speed
                };
                let time_limit = if cli.time_limit == DEFAULT_TIME_LIMIT {
                    0.0
                } else {
                    cli.time_limit
                };

                let guard = GuardState::install(&cli, &mut emu);
                let instructions = run_terminal_mode(
                    &mut emu,
                    speed,
                    time_limit,
                    cli.max_instructions,
                    cli.echo,
                    &cli.uart_input,
                    &guard,
                );

                eprintln!("Executed {} instructions", instructions);
                if cli.trace > 0 {
                    print!("{}", emu.trace().format_last(cli.trace));
                }
                if cli.dump {
                    print_dump(&emu, cli.dump_uart);
                }
                return;
            }

            let run_msg = format!(
                "Running (speed: {} IPS, time limit: {}s)...\n",
                if cli.speed == 0 {
                    "max".to_string()
                } else {
                    cli.speed.to_string()
                },
                cli.time_limit
            );
            if cli.quiet {
                eprintln!("{}", run_msg);
            } else {
                println!("{}", run_msg);
            }

            if cli.step {
                run_step_mode(&mut emu, cli.max_instructions, &cli.uart_input);
            } else {
                let guard = GuardState::install(&cli, &mut emu);
                let instructions = run_with_timing(
                    &mut emu,
                    cli.speed,
                    cli.time_limit,
                    cli.max_instructions,
                    &cli.uart_input,
                    cli.quiet,
                    &guard,
                );

                if !cli.quiet {
                    let uart = emu.get_uart_output();
                    if !uart.is_empty() {
                        println!("\nUART output: {}", uart);
                    }
                    println!("\nExecuted {} instructions", instructions);
                    if emu.is_halted() {
                        println!("CPU halted (self-branch detected)");
                    }
                } else {
                    eprintln!("\nExecuted {} instructions", instructions);
                    if emu.is_halted() {
                        eprintln!("CPU halted (self-branch detected)");
                    }
                }
            }
            if cli.trace > 0 {
                print!("{}", emu.trace().format_last(cli.trace));
            }
            if cli.dump {
                print_dump(&emu, cli.dump_uart);
            }
        }

        _ => {
            eprintln!("Unknown command. Use --demo, --run, --load-binary, or --assemble");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_prefix() {
        assert_eq!(parse_numeric_addr("0x010000"), Some(0x010000));
        assert_eq!(parse_numeric_addr("0X0"), Some(0));
        assert_eq!(parse_numeric_addr("0xFF"), Some(0xFF));
    }

    #[test]
    fn test_parse_hex_suffix() {
        assert_eq!(parse_numeric_addr("010000h"), Some(0x010000));
        assert_eq!(parse_numeric_addr("FFH"), Some(0xFF));
    }

    #[test]
    fn test_parse_decimal() {
        assert_eq!(parse_numeric_addr("0"), Some(0));
        assert_eq!(parse_numeric_addr("65536"), Some(65536));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(parse_numeric_addr("xyz"), None);
        assert_eq!(parse_numeric_addr(""), None);
    }

    #[test]
    fn test_p24_magic_detection() {
        let mut data = vec![0x50, 0x32, 0x34, 0x00];
        data.extend_from_slice(&[0; 14]);
        data.extend_from_slice(b"HELLO");
        assert_eq!(data.len(), 23);
        assert!(data.len() >= P24_HEADER_SIZE && data[..4] == P24_MAGIC);
        let body = &data[P24_HEADER_SIZE..];
        assert_eq!(body, b"HELLO");
    }

    #[test]
    fn test_raw_binary_no_strip() {
        let data = vec![0x44, 0x05, 0x5A];
        assert!(!(data.len() >= P24_HEADER_SIZE && data[..4] == P24_MAGIC));
    }

    #[test]
    fn test_patch_writes_24bit_le() {
        let mut emu = EmulatorCore::new();
        load_binaries_and_patches(&mut emu, &[], &[(0x100, 0x010000)]);
        assert_eq!(emu.read_byte(0x100), 0x00);
        assert_eq!(emu.read_byte(0x101), 0x00);
        assert_eq!(emu.read_byte(0x102), 0x01);
    }

    #[test]
    fn test_patch_multiple() {
        let mut emu = EmulatorCore::new();
        load_binaries_and_patches(&mut emu, &[], &[(0x100, 0xABCDEF), (0x200, 0x42)]);
        assert_eq!(emu.read_byte(0x100), 0xEF);
        assert_eq!(emu.read_byte(0x101), 0xCD);
        assert_eq!(emu.read_byte(0x102), 0xAB);
        assert_eq!(emu.read_byte(0x200), 0x42);
    }

    #[test]
    fn test_load_raw_binary_file() {
        let tmp = std::env::temp_dir().join("cor24_emu_test_raw.bin");
        fs::write(&tmp, &[0x44, 0x05, 0x5A]).unwrap();
        let mut emu = EmulatorCore::new();
        load_binaries_and_patches(
            &mut emu,
            &[(tmp.to_string_lossy().to_string(), 0x1000)],
            &[],
        );
        assert_eq!(emu.read_byte(0x1000), 0x44);
        assert_eq!(emu.read_byte(0x1001), 0x05);
        assert_eq!(emu.read_byte(0x1002), 0x5A);
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_load_p24_strips_header() {
        let tmp = std::env::temp_dir().join("cor24_emu_test.p24");
        let mut data = vec![0x50, 0x32, 0x34, 0x00]; // P24 magic
        data.extend_from_slice(&[0x01]); // version
        data.extend_from_slice(&[0x00, 0x00, 0x00]); // entry_point
        data.extend_from_slice(&[0x03, 0x00, 0x00]); // code_size
        data.extend_from_slice(&[0x00, 0x00, 0x00]); // data_size
        data.extend_from_slice(&[0x00, 0x00, 0x00]); // global_count
        data.push(0x00); // reserved
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // body
        assert_eq!(data.len(), P24_HEADER_SIZE + 3);
        fs::write(&tmp, &data).unwrap();

        let mut emu = EmulatorCore::new();
        load_binaries_and_patches(
            &mut emu,
            &[(tmp.to_string_lossy().to_string(), 0x010000)],
            &[],
        );
        assert_eq!(emu.read_byte(0x010000), 0xAA);
        assert_eq!(emu.read_byte(0x010001), 0xBB);
        assert_eq!(emu.read_byte(0x010002), 0xCC);
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_binary_mode_runs_program() {
        let mut asm = Assembler::new();
        let result = asm.assemble("lc r0, 42\nhalt:\n bra halt");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let bytes: Vec<u8> = result
            .lines
            .iter()
            .flat_map(|l| l.bytes.iter().copied())
            .collect();

        let mut emu = EmulatorCore::new();
        for (i, &b) in bytes.iter().enumerate() {
            emu.write_byte(i as u32, b);
        }
        emu.set_pc(0);
        emu.resume();
        emu.run_batch(100);
        let snap = emu.snapshot();
        assert_eq!(snap.regs[0], 42);
        assert!(snap.halted);
    }

    #[test]
    fn test_assemble_at_base_and_load() {
        let mut asm = Assembler::new();
        let code = "la r0, target\ntarget:\n  lc r1, 99\nhalt:\n  bra halt";
        let result = asm.assemble_at(code, 0x010000);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert_eq!(result.labels["target"], 0x010004);

        let mut emu = EmulatorCore::new();
        for (i, &b) in result.bytes.iter().enumerate() {
            emu.write_byte(0x010000 + i as u32, b);
        }
        emu.set_pc(0x010000);
        emu.resume();
        emu.run_batch(100);
        let snap = emu.snapshot();
        assert_eq!(snap.regs[0], 0x010004);
        assert_eq!(snap.regs[1], 99);
        assert!(snap.halted);
    }
}
