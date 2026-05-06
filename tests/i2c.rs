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
    // With Phase A.1+ stubs, the bus state machine tracks transitions
    // but no devices are attached, so every transaction NAKs. The CPU
    // should stay alive — not halt, not trip stack guards, not hit an
    // invalid instruction — for at least 100k instructions.
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

#[test]
fn tmp101_drives_bus_state_machine() {
    // Step A.3 observability: running the fixture should trigger at
    // least one START on the bus and the address byte should decode to
    // 0x4A (the TMP101's I2C address).
    let mut core = load_fixture();
    core.resume();
    let _ = core.run_batch(200_000);

    let i2c = core.i2c();
    assert!(
        i2c.transactions > 0,
        "expected at least one START detected; got {}",
        i2c.transactions,
    );
    assert_eq!(
        i2c.last_addressed,
        Some(0x4A),
        "expected the TMP101 address (0x4A) to have been observed",
    );
}
