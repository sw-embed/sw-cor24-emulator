//! Integration-test scaffold for I2C demos.
//!
//! Saga step 001 landed the fixture-loads check; step 002 adds the
//! stub-MMIO smoke test; step 005 adds the device-trait + add1 +
//! handle integration tests. Later steps attach TMP101 and assert
//! deterministic UART output for a configured temperature.

use cor24_emulator::peripherals::i2c::{Add1Device, Tmp101Device, Tmp101HandleExt};
use cor24_emulator::{EmulatorCore, StopReason};

const IO_I2C_SCL: u32 = 0xFF0020;
const IO_I2C_SDA: u32 = 0xFF0021;

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

/// Direct-bus harness that drives the I2C protocol through `EmulatorCore`'s
/// MMIO without running a CPU program. The CPU+libi2c bit-banging path
/// is exercised by the tmp101.lgo fixture once the TMP101 device lands
/// in the next step; for now this is the canonical end-to-end shape.
fn bus_start(emu: &mut EmulatorCore) {
    emu.write_byte(IO_I2C_SDA, 1);
    emu.write_byte(IO_I2C_SCL, 1);
    emu.write_byte(IO_I2C_SDA, 0);
    emu.write_byte(IO_I2C_SCL, 0);
}

fn bus_stop(emu: &mut EmulatorCore) {
    emu.write_byte(IO_I2C_SDA, 0);
    emu.write_byte(IO_I2C_SCL, 1);
    emu.write_byte(IO_I2C_SDA, 1);
}

fn bus_write_byte(emu: &mut EmulatorCore, byte: u8) -> bool {
    for i in (0..8).rev() {
        emu.write_byte(IO_I2C_SDA, (byte >> i) & 1);
        emu.write_byte(IO_I2C_SCL, 1);
        emu.write_byte(IO_I2C_SCL, 0);
    }
    emu.write_byte(IO_I2C_SDA, 1);
    emu.write_byte(IO_I2C_SCL, 1);
    let acked = emu.read_byte(IO_I2C_SDA) == 0;
    emu.write_byte(IO_I2C_SCL, 0);
    acked
}

fn bus_read_byte(emu: &mut EmulatorCore, master_acks: bool) -> u8 {
    let mut byte = 0u8;
    for _ in 0..8 {
        emu.write_byte(IO_I2C_SDA, 1);
        emu.write_byte(IO_I2C_SCL, 1);
        byte = (byte << 1) | (emu.read_byte(IO_I2C_SDA) & 1);
        emu.write_byte(IO_I2C_SCL, 0);
    }
    emu.write_byte(IO_I2C_SDA, if master_acks { 0 } else { 1 });
    emu.write_byte(IO_I2C_SCL, 1);
    emu.write_byte(IO_I2C_SCL, 0);
    byte
}

#[test]
fn add1_write_then_read_increments_through_bus() {
    // The plan-§7 layer-3 test for step 005: write addr / write byte /
    // STOP; START / read addr / read / read / STOP. The second read
    // must come back as last_written + 2.
    let mut emu = EmulatorCore::new();
    let _h = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();

    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, 0x50 << 1), "addr write should ACK");
    assert!(bus_write_byte(&mut emu, 0x42), "data byte should ACK");
    bus_stop(&mut emu);

    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, (0x50 << 1) | 1), "addr read should ACK");
    let r1 = bus_read_byte(&mut emu, true); // master ACKs to keep going
    let r2 = bus_read_byte(&mut emu, false); // master NAKs to end
    bus_stop(&mut emu);

    assert_eq!(r1, 0x43, "first read = last_written + 1");
    assert_eq!(r2, 0x44, "second read = last_written + 2");
}

#[test]
fn handle_with_round_trip_visible_to_bus() {
    // handle.with(|d| d.poke(...)) updates the device, and a subsequent
    // bus read sees the new value through the same Arc<Mutex<...>>.
    let mut emu = EmulatorCore::new();
    let h = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();
    h.with(|d| d.poke(0x10));

    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, (0x50 << 1) | 1));
    let r = bus_read_byte(&mut emu, false);
    bus_stop(&mut emu);
    assert_eq!(r, 0x11, "poked value should drive next read");
}

#[test]
fn handle_set_address_reroutes_bus() {
    let mut emu = EmulatorCore::new();
    let h = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();

    // Initially at 0x50: ACKs.
    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, 0x50 << 1));
    bus_stop(&mut emu);

    // Move to 0x42.
    h.set_address(0x42).unwrap();
    assert_eq!(h.address(), 0x42);

    // 0x50 now NAKs; 0x42 ACKs.
    bus_start(&mut emu);
    assert!(!bus_write_byte(&mut emu, 0x50 << 1), "old addr should NAK");
    bus_stop(&mut emu);

    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, 0x42 << 1), "new addr should ACK");
    bus_stop(&mut emu);
}

#[test]
fn handle_set_address_rejects_collision() {
    use cor24_emulator::peripherals::i2c::AddressInUse;

    let mut emu = EmulatorCore::new();
    let _h1 = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();
    let h2 = emu.attach_i2c_device(Add1Device::new(0x42, 0x100)).unwrap();

    // Move h2 onto h1's address — should reject.
    let err = h2
        .set_address(0x50)
        .expect_err("collision must be rejected");
    assert_eq!(err, AddressInUse { address: 0x50 });
    // h2 still at 0x42.
    assert_eq!(h2.address(), 0x42);
}

#[test]
fn attach_rejects_duplicate_address() {
    use cor24_emulator::peripherals::i2c::AddressInUse;

    let mut emu = EmulatorCore::new();
    let _h1 = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();
    let err = emu
        .attach_i2c_device(Add1Device::new(0x50, 0x100))
        .expect_err("duplicate attach must be rejected");
    assert_eq!(err, AddressInUse { address: 0x50 });
}

#[test]
fn detach_clears_routing() {
    let mut emu = EmulatorCore::new();
    let _h = emu.attach_i2c_device(Add1Device::new(0x50, 0x100)).unwrap();

    bus_start(&mut emu);
    assert!(bus_write_byte(&mut emu, 0x50 << 1));
    bus_stop(&mut emu);

    emu.detach_i2c_devices();

    bus_start(&mut emu);
    assert!(!bus_write_byte(&mut emu, 0x50 << 1), "after detach, NAK");
    bus_stop(&mut emu);
}

#[test]
fn tmp101_lgo_prints_configured_temperature() {
    // Plan §7 layer-3 e2e test: load tmp101.lgo, attach a Tmp101 at
    // 0x4A configured to 25.0°C, run, expect "25.00\n" in the UART.
    // This exercises the entire stack — libi2c bit-banging, bus
    // state machine, slave_sda_pull wired-AND, Tmp101 register file,
    // resolution-aware temp_register encoding, the demo's hand-rolled
    // printf — and is the single most important test in this saga.
    let mut emu = load_fixture();
    let h = emu.attach_i2c_device(Tmp101Device::new(0x4A)).unwrap();
    h.set_temperature(25.0);

    emu.resume();
    let _ = emu.run_batch(2_000_000);

    let out = emu.get_uart_output();
    assert!(
        out.contains("25.00\n"),
        "expected '25.00\\n' in UART output, got {out:?}",
    );
}

#[test]
fn tmp101_lgo_prints_negative_temperature() {
    let mut emu = load_fixture();
    let h = emu.attach_i2c_device(Tmp101Device::new(0x4A)).unwrap();
    h.set_temperature(-12.5);

    emu.resume();
    let _ = emu.run_batch(2_000_000);

    let out = emu.get_uart_output();
    assert!(
        out.contains("-12.50\n"),
        "expected '-12.50\\n' in UART output, got {out:?}",
    );
}

// (Mid-run handle.set_temperature mutation is exercised at lib level
// by handle_with_round_trip_visible_to_bus and at unit level by
// Tmp101Device tests. End-to-end "Web UI moves the slider, the next
// printtemp sees the new value" needs the demo's 16M-iteration delay
// loop to elapse between reads, which is too slow for a unit test.
// The CLI-step's web-surface-smoke fixture will pin the API shape.)

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
