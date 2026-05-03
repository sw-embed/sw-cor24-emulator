//! Integration-test scaffold for I2C demos.
//!
//! Saga step 001 lands only the fixture-loads check. Later steps add
//! bus-attached devices and assert UART output for a configured
//! temperature.

use cor24_emulator::emulator::EmulatorCore;

#[test]
fn tmp101_fixture_loads() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/i2c/tmp101/tmp101.lgo"
    );
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Cannot read {path}: {e}"));

    let mut core = EmulatorCore::new();
    let bytes = core
        .load_lgo(&content, None)
        .unwrap_or_else(|e| panic!("Failed to load tmp101.lgo: {e}"));

    assert!(bytes > 0, "Expected non-empty load, got {bytes} bytes");
}
