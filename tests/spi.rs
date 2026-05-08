//! Integration-test scaffold for SPI demos.
//!
//! Saga step 001 (this file) lands the fixture-loads check + a
//! crash-free smoke run with no SPI bus emulation. With no SPI MMIO
//! state yet, the guest spins in `spixchg` waiting for SCLK reads to
//! return what it just wrote — but it neither halts nor executes an
//! invalid instruction, which is what the smoke test asserts.
//!
//! Later steps add the SPI MMIO state, the shift-register bus model,
//! and the `SpiDevice` trait + TMP125 device — at which point a third
//! e2e test will assert deterministic UART output (`"DD.DD\n"`) for a
//! configured temperature.

use cor24_emulator::{EmulatorCore, StopReason};

const TMP125_LGO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/spi/tmp125/tmp125.lgo"
);

fn load_fixture() -> EmulatorCore {
    let content = std::fs::read_to_string(TMP125_LGO)
        .unwrap_or_else(|e| panic!("Cannot read {TMP125_LGO}: {e}"));
    let mut core = EmulatorCore::new();
    core.load_lgo(&content, None)
        .unwrap_or_else(|e| panic!("Failed to load tmp125.lgo: {e}"));
    core
}

#[test]
fn tmp125_fixture_loads() {
    let content = std::fs::read_to_string(TMP125_LGO).unwrap();
    let mut core = EmulatorCore::new();
    let bytes = core.load_lgo(&content, None).unwrap();
    assert!(bytes > 0, "Expected non-empty load, got {bytes} bytes");
}

#[test]
fn tmp125_runs_with_stub_mmio() {
    // No SPI bus state machine yet — the guest's spixchg loop reads
    // back whatever it last wrote to the SCLK / SELN MMIO addresses
    // (which today are unmapped and read 0). The CPU should stay alive
    // for at least 100k instructions: not halt, not trip stack guards,
    // not execute an invalid opcode.
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
