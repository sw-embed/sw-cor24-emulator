//! Integration-test scaffold for I2C demos.
//!
//! Saga step 001 landed the fixture-loads check; step 002 adds the
//! stub-MMIO smoke test. Later steps attach devices and assert
//! deterministic UART output for a configured temperature.

use cor24_emulator::{EmulatorCore, StopReason};

const TMP101_LGO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/i2c/tmp101/tmp101.lgo"
);

fn load_fixture() -> EmulatorCore {
    let content = std::fs::read_to_string(TMP101_LGO)
        .unwrap_or_else(|e| panic!("Cannot read {TMP101_LGO}: {e}"));
    let mut core = EmulatorCore::new();
    core.load_lgo(&content, None)
        .unwrap_or_else(|e| panic!("Failed to load tmp101.lgo: {e}"));
    core
}

#[test]
fn tmp101_fixture_loads() {
    let content = std::fs::read_to_string(TMP101_LGO).unwrap();
    let mut core = EmulatorCore::new();
    let bytes = core.load_lgo(&content, None).unwrap();
    assert!(bytes > 0, "Expected non-empty load, got {bytes} bytes");
}

#[test]
fn tmp101_runs_with_stub_mmio() {
    // With Phase A.1 stubs, I2C reads always return 1 (idle high) and
    // writes are no-ops. The demo's transactions still complete (every
    // slave NAK reads as wired-AND high), so the program should make
    // steady progress and the CPU should stay alive — not halt, not
    // trip the stack guards, not hit an invalid instruction.
    let mut core = load_fixture();
    core.resume();
    let result = core.run_batch(100_000);

    assert_eq!(
        result.reason,
        StopReason::CycleLimit,
        "stub MMIO should not crash or halt; got {:?} after {} instructions",
        result.reason,
        result.instructions_run,
    );
    assert_eq!(result.instructions_run, 100_000);
}
