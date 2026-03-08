//! Integration tests for COR24 emulator using as24-assembled .lgo files

use cor24_emulator::cpu::executor::Executor;
use cor24_emulator::cpu::state::CpuState;
use cor24_emulator::loader::load_lgo;

/// Load an LGO file, set PC, run for max_cycles
fn load_and_run(lgo_path: &str, entry: u32, max_cycles: u64) -> CpuState {
    let content = std::fs::read_to_string(lgo_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", lgo_path, e));
    let mut cpu = CpuState::new();
    load_lgo(&content, &mut cpu).unwrap();
    cpu.pc = entry;
    let executor = Executor::new();
    executor.run(&mut cpu, max_cycles);
    cpu
}

#[test]
fn test_led_on() {
    let cpu = load_and_run(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs/led_on.lgo"),
        0,
        100,
    );
    assert_eq!(cpu.io.leds, 0x01, "LED D2 should be on (bit 0 = 1)");
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
        concat!(env!("CARGO_MANIFEST_DIR"), "/docs/research/asld24/sieve.lgo"),
        0x93, // _main entry point
        500_000_000,
    );
    assert_eq!(
        cpu.io.uart_output,
        "1000 iterations\n1899 primes.\n",
        "Sieve should produce correct output"
    );
}
