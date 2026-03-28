//! Integration tests for COR24 emulator using as24-assembled .lgo files

use cor24_emulator::assembler::Assembler;
use cor24_emulator::challenge::get_examples;
use cor24_emulator::cpu::executor::Executor;
use cor24_emulator::cpu::state::CpuState;
use cor24_emulator::loader::load_lgo;

/// Load an LGO file, set PC, run for max_cycles
fn load_and_run(lgo_path: &str, entry: u32, max_cycles: u64) -> CpuState {
    let content = std::fs::read_to_string(lgo_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", lgo_path, e));
    let mut cpu = CpuState::new();
    cpu.io.uart_tx_busy_cycles = 0; // legacy: instant TX for .lgo programs that don't poll
    load_lgo(&content, &mut cpu).unwrap();
    cpu.pc = entry;
    let executor = Executor::new();
    executor.run(&mut cpu, max_cycles);
    cpu
}

#[test]
fn test_led_on_lgo() {
    // The reference as24 .lgo writes 1 to LED register.
    // On hardware this is LED OFF (active-low: 1=OFF).
    // We preserve the .lgo as-is — it's from the reference toolchain.
    let cpu = load_and_run(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs/led_on.lgo"),
        0,
        100,
    );
    assert_eq!(cpu.io.leds, 0x01, "LED register should be 0x01 (LED OFF, active-low)");
}

#[test]
fn test_hello_uart() {
    let cpu = load_and_run(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs/hello_uart.lgo"),
        0,
        100,
    );
    assert_eq!(cpu.io.uart_output, "Hi\n", "UART should output 'Hi\\n'");
}

#[test]
fn test_count_down() {
    let cpu = load_and_run(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs/count_down.lgo"),
        0,
        1000,
    );
    assert_eq!(cpu.io.uart_output, "54321", "Should count down from 5 to 1");
}

#[test]
fn test_hello_world() {
    let cpu = load_and_run(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/programs/hello_world.lgo"
        ),
        0,
        1000,
    );
    assert_eq!(
        cpu.io.uart_output, "Hello, World!\n",
        "Should print 'Hello, World!\\n'"
    );
}

#[test]
fn test_led_blink() {
    let cpu = load_and_run(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs/led_blink.lgo"),
        0,
        100_000,
    );
    assert_eq!(cpu.io.uart_output, "LLLLL", "Should print 'L' five times");
}

#[test]
fn test_sieve() {
    let cpu = load_and_run(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/research/asld24/sieve.lgo"
        ),
        0x93, // _main entry point
        1_000_000,
    );
    assert!(
        cpu.io.uart_output.starts_with("1000 iterations"),
        "Sieve should start printing iteration count, got: {:?}",
        cpu.io.uart_output
    );
}

#[test]
fn test_all_examples_assemble() {
    let examples = get_examples();
    for (name, _desc, source) in &examples {
        let mut assembler = Assembler::new();
        let result = assembler.assemble(source);
        assert!(
            result.errors.is_empty(),
            "Example '{}' failed to assemble: {:?}",
            name,
            result.errors
        );
    }
}

/// Test Fibonacci example prints correct series to UART
#[test]
fn test_fibonacci_example() {
    let mut assembler = Assembler::new();
    let examples = get_examples();
    let fib = examples
        .iter()
        .find(|(name, _, _)| name == "Fibonacci")
        .unwrap();
    let result = assembler.assemble(&fib.2);
    assert!(
        result.errors.is_empty(),
        "Fibonacci assembly errors: {:?}",
        result.errors
    );
    let mut cpu = CpuState::new();
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 100_000);
    assert_eq!(
        cpu.io.uart_output, "1 1 2 3 5 8 13 21 34 55\n",
        "Fibonacci should print series"
    );
}

/// Test Multiply example prints correct result to UART
#[test]
fn test_multiply_example() {
    let mut assembler = Assembler::new();
    let examples = get_examples();
    let mul = examples
        .iter()
        .find(|(name, _, _)| name == "Multiply")
        .unwrap();
    let result = assembler.assemble(&mul.2);
    assert!(
        result.errors.is_empty(),
        "Multiply assembly errors: {:?}",
        result.errors
    );
    let mut cpu = CpuState::new();
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 10_000);
    assert_eq!(
        cpu.io.uart_output, "42 42\n",
        "Multiply should print '42 42\\n'"
    );
}

/// Helper: assemble source, run, and return CPU state.
/// Uses instant UART TX (busy_cycles=0) for legacy tests that don't poll.
fn assemble_and_run(source: &str, max_cycles: u64) -> CpuState {
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(
        result.errors.is_empty(),
        "Assembly errors: {:?}",
        result.errors
    );
    let mut cpu = CpuState::new();
    cpu.io.uart_tx_busy_cycles = 0; // legacy: instant TX for tests that don't poll
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, max_cycles);
    cpu
}

/// All examples that terminate must reach halted state
#[test]
fn test_all_examples_halt() {
    // Examples that intentionally loop forever (no halt)
    let non_halting = ["Blink LED", "Button Echo", "Echo", "Loop Trace"];
    let examples = get_examples();
    for (name, _desc, source) in &examples {
        if non_halting.contains(&name.as_str()) {
            continue;
        }
        let mut assembler = Assembler::new();
        let result = assembler.assemble(source);
        if !result.errors.is_empty() {
            continue; // skip broken examples (tested elsewhere)
        }
        let mut cpu = CpuState::new();
        for (addr, byte) in result.bytes.iter().enumerate() {
            cpu.memory[addr] = *byte;
        }
        cpu.pc = 0;
        let executor = Executor::new();
        executor.run(&mut cpu, 500_000);
        assert!(
            cpu.halted,
            "Example '{}' did not halt within 500K cycles (PC=0x{:06X})",
            name, cpu.pc
        );
    }
}

/// Self-branch halt detection works via single-step
#[test]
fn test_self_branch_halt_via_step() {
    let cpu = assemble_and_run("lc r0,1\nhalt:\nbra halt", 100);
    assert!(cpu.halted, "Self-branch should be detected as halt");
    assert_eq!(cpu.pc, 0x0002, "PC should point at the bra instruction");
}

/// Step past halt: stepping a halted CPU should not change state
#[test]
fn test_step_halted_cpu_is_noop() {
    let mut cpu = assemble_and_run("halt:\nbra halt", 100);
    assert!(cpu.halted);
    let pc_before = cpu.pc;
    let cycles_before = cpu.cycles;
    let executor = Executor::new();
    executor.step(&mut cpu);
    assert!(cpu.halted, "CPU should remain halted after step");
    assert_eq!(cpu.pc, pc_before, "PC should not change");
    assert_eq!(cpu.cycles, cycles_before, "Cycles should not change");
}

/// Memory Access example stores to non-adjacent blocks
#[test]
fn test_memory_access_non_adjacent() {
    let examples = get_examples();
    let mem = examples
        .iter()
        .find(|(name, _, _)| name == "Memory Access")
        .unwrap();
    let cpu = assemble_and_run(&mem.2, 1000);
    assert!(cpu.halted, "Memory Access should halt");
    // Check first block at 0x0100
    assert_eq!(cpu.read_byte(0x0100), 42, "Block 1: byte 0 should be 42");
    assert_eq!(cpu.read_byte(0x0101), 42, "Block 1: byte 1 should be 42");
    // Check second block at 0x0200
    assert_eq!(cpu.read_byte(0x0200), 200, "Block 2: byte 0 should be 200");
    // Gap between blocks should be zero
    assert_eq!(
        cpu.read_byte(0x0150),
        0,
        "Gap between blocks should be zero"
    );
}

/// Test that UART Hello example with TX busy polling assembles and runs correctly
#[test]
fn test_uart_hello_example() {
    let mut assembler = Assembler::new();
    let examples = get_examples();
    let uart = examples
        .iter()
        .find(|(name, _, _)| name == "UART Hello")
        .unwrap();
    let result = assembler.assemble(&uart.2);
    assert!(
        result.errors.is_empty(),
        "UART Hello assembly errors: {:?}",
        result.errors
    );
    let mut cpu = CpuState::new();
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 10_000);
    assert_eq!(
        cpu.io.uart_output, "Hello\n",
        "UART Hello should output 'Hello\\n'"
    );
}

/// OOM example fills SRAM with 256-byte stride then halts
#[test]
fn test_oom_example() {
    let source = include_str!("../docs/examples/oom.s");
    let cpu = assemble_and_run(source, 500_000);
    assert!(cpu.halted, "OOM should halt when SRAM is exhausted");
    // First write at 0x0100 should be counter value 1
    assert_eq!(cpu.read_byte(0x0100), 1, "First store should be 1");
    // Second write at 0x0200 should be counter value 2
    assert_eq!(cpu.read_byte(0x0200), 2, "Second store should be 2");
    // Gap between writes should be zero
    assert_eq!(cpu.read_byte(0x0150), 0, "Gap should be zero");
}

/// Stack overflow example fills EBR/Stack region then halts
#[test]
fn test_stack_overflow_example() {
    let source = include_str!("../docs/examples/stack_overflow.s");
    let cpu = assemble_and_run(source, 500_000);
    assert!(cpu.halted, "Stack overflow should halt when EBR exhausted");
    // SP should be at or below EBR base (0xFEE000)
    let sp = cpu.get_reg(4);
    assert!(
        sp <= 0xFEE000,
        "SP should be at or below EBR base, got 0x{:06X}",
        sp
    );
    // First push writes at 0xFEEBFD (SP-3 from initial 0xFEEC00)
    // Depth 0 is pushed, so word value is 0 — check second push pair (depth=1 at offset -12)
    assert_ne!(
        cpu.read_word(0xFEEBF1),
        0,
        "Stack should have recursion data"
    );
}

/// Interrupt example: send UART bytes, verify ISR prints counter digits
#[test]
fn test_interrupt_example() {
    let source = include_str!("../docs/examples/interrupt.s");
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(
        result.errors.is_empty(),
        "Interrupt assembly errors: {:?}",
        result.errors
    );

    let mut cpu = CpuState::new();
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();

    // Run some cycles to let main loop start counting
    executor.run(&mut cpu, 1000);
    assert!(!cpu.halted, "Main loop should keep running");

    // Send a UART byte to trigger interrupt
    cpu.uart_send_rx(b'x');
    executor.run(&mut cpu, 1000);

    // ISR should have printed a digit (0-9) to UART
    assert!(
        !cpu.io.uart_output.is_empty(),
        "ISR should have output a digit"
    );
    let first_char = cpu.io.uart_output.chars().next().unwrap();
    assert!(
        first_char.is_ascii_digit(),
        "Output should be ASCII digit, got '{}'",
        first_char
    );

    // Send another byte, should get another digit
    cpu.uart_send_rx(b'y');
    executor.run(&mut cpu, 1000);
    assert_eq!(
        cpu.io.uart_output.len(),
        2,
        "Should have two digits after two interrupts"
    );
}

/// Echo example: letters→uppercase, !→halt, others echo as-is
#[test]
fn test_echo_example() {
    let source = include_str!("../src/examples/assembler/echo.s");
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(
        result.errors.is_empty(),
        "Echo assembly errors: {:?}",
        result.errors
    );

    let mut cpu = CpuState::new();
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();

    // Run to reach idle loop — prompt '?' should appear
    executor.run(&mut cpu, 100);
    assert_eq!(
        cpu.io.uart_output, "?",
        "Prompt '?' should appear on startup"
    );

    // Send 'a' → uppercase 'A'
    cpu.uart_send_rx(b'a');
    executor.run(&mut cpu, 1000);
    assert_eq!(cpu.io.uart_output, "?A", "'a' -> 'A'");

    // Send 'B' → already uppercase, echo 'B'
    cpu.uart_send_rx(b'B');
    executor.run(&mut cpu, 1000);
    assert_eq!(cpu.io.uart_output, "?AB", "'B' -> 'B'");

    // Send '1' → not a letter, echo as-is
    cpu.uart_send_rx(b'1');
    executor.run(&mut cpu, 1000);
    assert_eq!(cpu.io.uart_output, "?AB1", "'1' -> '1'");

    // Send '!' → halts
    cpu.uart_send_rx(b'!');
    executor.run(&mut cpu, 1000);
    assert!(cpu.halted, "Should halt on '!'");
}

// ===== UART TX Discipline Tests =====

/// Program that writes to UART WITHOUT polling TX busy.
/// With realistic timing (busy_cycles=10), characters should be dropped.
#[test]
fn test_uart_no_poll_drops_characters() {
    let source = r#"
        la      r1,-65280
        lc      r0,65
        sb      r0,0(r1)
        lc      r0,66
        sb      r0,0(r1)
        lc      r0,67
        sb      r0,0(r1)
halt:
        bra     halt
    "#;
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(result.errors.is_empty());
    let mut cpu = CpuState::new();
    // Realistic: 10 cycles busy after each write
    cpu.io.uart_tx_busy_cycles = 10;
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 10_000);
    // Only first character should get through — B and C written while busy
    assert_eq!(
        cpu.io.uart_output, "A",
        "Only 'A' should transmit; B,C dropped while busy"
    );
    assert_eq!(cpu.io.uart_tx_dropped, 2, "B and C should be dropped");
}

/// Program that correctly polls TX busy before each write.
/// All characters should transmit even with realistic timing.
#[test]
fn test_uart_with_poll_all_characters() {
    let source = r#"
        la      r1,-65280
        lc      r0,65
.w1:
        lb      r2,1(r1)
        cls     r2,z
        brt     .w1
        sb      r0,0(r1)
        lc      r0,66
.w2:
        lb      r2,1(r1)
        cls     r2,z
        brt     .w2
        sb      r0,0(r1)
        lc      r0,67
.w3:
        lb      r2,1(r1)
        cls     r2,z
        brt     .w3
        sb      r0,0(r1)
halt:
        bra     halt
    "#;
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(result.errors.is_empty());
    let mut cpu = CpuState::new();
    cpu.io.uart_tx_busy_cycles = 10;
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 10_000);
    assert_eq!(
        cpu.io.uart_output, "ABC",
        "All characters should transmit with polling"
    );
    assert_eq!(cpu.io.uart_tx_dropped, 0, "No characters should be dropped");
}

/// With uart_never_ready, a polling program should hang (not halt).
#[test]
fn test_uart_never_ready_hangs_polling_program() {
    let source = r#"
        la      r1,-65280
.wait:
        lb      r2,1(r1)
        cls     r2,z
        brt     .wait
        lc      r0,65
        sb      r0,0(r1)
halt:
        bra     halt
    "#;
    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);
    assert!(result.errors.is_empty());
    let mut cpu = CpuState::new();
    cpu.io.uart_never_ready = true;
    cpu.io.uart_tx_busy = true; // start busy
    for (addr, byte) in result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;
    let executor = Executor::new();
    executor.run(&mut cpu, 10_000);
    // Should NOT have halted — stuck in .wait loop
    assert!(!cpu.halted, "Should be stuck polling, not halted");
    assert_eq!(cpu.io.uart_output, "", "No output — never got past poll");
}
