//! cor24-run: COR24 assembler and emulator CLI
//!
//! Usage:
//!   cor24-run --demo                              Run built-in LED demo
//!   cor24-run --demo --speed 50000 --time 10      Run at 50k IPS for 10 seconds
//!   cor24-run --run <file.s>                      Assemble and run
//!   cor24-run --assemble <in.s> <out.bin> <out.lst>  Assemble to binary + listing

use cor24_emulator::assembler::Assembler;
use cor24_emulator::emulator::EmulatorCore;
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

/// Run emulator with timing control and optional instruction limit
fn run_with_timing(emu: &mut EmulatorCore, speed: u64, time_limit: f64, max_instructions: i64) -> u64 {
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
            "--entry" | "-e" => {
                if i + 1 < args.len() {
                    cli.entry = Some(args[i + 1].clone());
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
        println!("  --dump               Dump CPU state, I/O, and non-zero memory after halt");
        println!("  --entry, -e <label>  Set entry point to label address");
        println!();
        println!("Example:");
        println!("  cor24-run --demo --speed 100000 --time 10");
        println!("  cor24-run --run prog.s --dump --speed 0");
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
            let instructions = run_with_timing(&mut emu, cli.speed, cli.time_limit, cli.max_instructions);

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

            println!("Running (speed: {} IPS, time limit: {}s)...\n",
                     if cli.speed == 0 { "max".to_string() } else { cli.speed.to_string() },
                     cli.time_limit);

            let instructions = run_with_timing(&mut emu, cli.speed, cli.time_limit, cli.max_instructions);

            // Print UART output if any
            let uart = emu.get_uart_output();
            if !uart.is_empty() {
                println!("\nUART output: {}", uart);
            }

            println!("\nExecuted {} instructions", instructions);
            if emu.is_halted() {
                println!("CPU halted (self-branch detected)");
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
