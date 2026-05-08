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
    // back whatever it last wrote to the SCLK / SELN MMIO addresses.
    // The CPU should stay alive for at least 100k instructions: not
    // halt, not trip stack guards, not execute an invalid opcode.
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
fn tmp125_drives_some_clocks() {
    // After Phase C.2 the master-line state persists writes, so
    // running the fixture for a while should cover both edges of the
    // SCLK line at least once. Confirms the spixchg loop is making
    // bus progress.
    //
    // Sample at fine granularity early in the run — spixchg takes
    // ~24 instructions per bit × 8 bits, so a 4-instruction stride
    // catches mid-bit-cycle states where SCLK has just been driven
    // 0 and not yet driven 1 again. After spixchg completes the
    // demo enters a 16M-iter delay loop with SCLK pinned at whatever
    // it was last; sampling there alone would miss the low half.
    const IO_SPI_SCLK: u32 = 0xFF0031;
    const IO_SPI_SELN: u32 = 0xFF0032;

    let mut core = load_fixture();
    core.resume();

    let mut saw_sclk_high = false;
    let mut saw_sclk_low = false;
    let mut saw_seln_low = false; // active-low: low = selected

    for _ in 0..1_500 {
        core.run_batch(4);
        if core.read_byte(IO_SPI_SCLK) == 1 {
            saw_sclk_high = true;
        } else {
            saw_sclk_low = true;
        }
        if core.read_byte(IO_SPI_SELN) == 0 {
            saw_seln_low = true;
        }
        if saw_sclk_high && saw_sclk_low && saw_seln_low {
            return; // early-out: all observations satisfied
        }
    }

    assert!(saw_sclk_low, "expected SCLK to be observed low at least once");
    assert!(saw_sclk_high, "expected SCLK to be observed high at least once");
    assert!(
        saw_seln_low,
        "expected SELN to be driven low (slave selected) at least once"
    );
}
