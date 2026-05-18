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

use cor24_emulator::peripherals::spi::{Tmp125Device, Tmp125HandleExt};
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

#[test]
fn tmp125_exchanges_bytes() {
    // Phase C.3 wired the shift register to write_io. After running
    // the fixture long enough for spixchg to clock 8 bits, the bus's
    // bytes_exchanged counter must be at least 1 and last_mosi_byte
    // must be Some(_).
    //
    // Phase C.4 (this step) introduced device dispatch. The fixture
    // here doesn't attach a device, so last_miso_byte is Some(0) —
    // but with a slave attached it would be the slave-driven byte.
    // Assert is_some() so the e2e harness in Phase C.5 stays
    // compatible with an attached TMP125 device returning non-zero.
    let mut core = load_fixture();
    core.resume();
    let _ = core.run_batch(50_000);

    let spi = core.spi();
    assert!(
        spi.bytes_exchanged >= 1,
        "expected at least one byte exchanged, got {}",
        spi.bytes_exchanged,
    );
    assert!(
        spi.last_mosi_byte.is_some(),
        "expected last_mosi_byte to be set after a byte exchange",
    );
    assert!(
        spi.last_miso_byte.is_some(),
        "expected last_miso_byte to be set after a byte exchange",
    );
}

#[test]
fn echo_device_observes_mosi_through_emulator() {
    // Synthetic-bus E2E: attach an EchoDevice, hand-drive SCLK / SELN
    // / DATA from outside the CPU, and assert the bus's MISO byte
    // sequence matches the echo pattern (seed → previous MOSI byte
    // each subsequent exchange). Mirrors the i2c
    // `add1_full_write_then_read_cycle` synthetic-bus pattern.
    use cor24_emulator::peripherals::spi::EchoDevice;

    const IO_SPI_SCLK: u32 = 0xFF0031;
    const IO_SPI_SELN: u32 = 0xFF0032;

    let mut core = EmulatorCore::new();
    let _handle = core.attach_spi_device(EchoDevice::new(0xC3));

    // Idle high; select.
    core.write_byte(IO_SPI_SELN, 1);
    core.write_byte(IO_SPI_SCLK, 0);
    core.write_byte(IO_SPI_SELN, 0);

    // Clock byte 0x11. EchoDevice on_select returned 0xC3 → expect
    // last_miso_byte = Some(0xC3).
    clock_byte(&mut core, 0x11);
    let spi = core.spi();
    assert_eq!(spi.last_mosi_byte, Some(0x11));
    assert_eq!(spi.last_miso_byte, Some(0xC3));

    // Clock byte 0x22. on_byte(0x11) returned 0x11 (echo of last
    // MOSI), but the last_miso_byte for the just-finished exchange
    // is still the byte the slave was driving across that exchange.
    // After clocking 0x22, last_miso_byte should be 0x11 (the byte
    // latched into shift_out at the start of this exchange).
    clock_byte(&mut core, 0x22);
    let spi = core.spi();
    assert_eq!(spi.last_mosi_byte, Some(0x22));
    assert_eq!(spi.last_miso_byte, Some(0x11));

    // Clock byte 0x33. last_miso_byte = 0x22 (echo of previous MOSI).
    clock_byte(&mut core, 0x33);
    let spi = core.spi();
    assert_eq!(spi.last_mosi_byte, Some(0x33));
    assert_eq!(spi.last_miso_byte, Some(0x22));

    // Deselect.
    core.write_byte(IO_SPI_SCLK, 0);
    core.write_byte(IO_SPI_SELN, 1);
}

/// Drive one SPI byte through the emulator's MMIO interface, MSB-first.
/// SELN is assumed to already be low; SCLK starts low.
fn clock_byte(core: &mut EmulatorCore, mosi_byte: u8) {
    const IO_SPI_DATA: u32 = 0xFF0030;
    const IO_SPI_SCLK: u32 = 0xFF0031;
    for i in (0..8).rev() {
        let bit = (mosi_byte >> i) & 1;
        core.write_byte(IO_SPI_DATA, bit);
        core.write_byte(IO_SPI_SCLK, 1);
        core.write_byte(IO_SPI_SCLK, 0);
    }
}

#[test]
fn tmp125_lgo_prints_configured_temperature() {
    // Plan §7 layer-3 e2e: load tmp125.lgo, attach a Tmp125 device
    // configured to 25.0°C, run, expect "25.00\n" in the UART. This
    // exercises the entire SPI stack — libspi bit-banging via spixchg,
    // bus state machine with MISO bit capture, Tmp125 register
    // synthesis, and the demo's hand-rolled printf — and is the
    // single most important test for this saga.
    let mut core = load_fixture();
    let h = core.attach_spi_device(Tmp125Device::new());
    h.set_temperature(25.0);

    core.resume();
    let _ = core.run_batch(2_000_000);

    let out = core.get_uart_output();
    assert!(
        out.contains("25.00\n"),
        "expected '25.00\\n' in UART output, got {out:?}",
    );
}

#[test]
fn tmp125_lgo_prints_negative_temperature() {
    let mut core = load_fixture();
    let h = core.attach_spi_device(Tmp125Device::new());
    h.set_temperature(-12.5);

    core.resume();
    let _ = core.run_batch(2_000_000);

    let out = core.get_uart_output();
    assert!(
        out.contains("-12.50\n"),
        "expected '-12.50\\n' in UART output, got {out:?}",
    );
}

/// SD card attached via the registry spec must respond to CMD17 with
/// the host-backed image's bytes. This is the integration test brief
/// 8 §Step 1 asks for. (`--spi-device` CLI plumbing is a separate
/// follow-up; this test feeds the same Rust path the CLI would use.)
#[test]
fn cli_sdcard_attach_reads_known_sector() {
    use cor24_emulator::peripherals::spi::build_spi_device;
    use std::fs;

    let path = std::env::temp_dir().join(format!(
        "cor24-sdcard-integration-{}.img",
        std::process::id(),
    ));
    let mut img = vec![0u8; 4 * 512];
    let pattern: Vec<u8> = (0..512).map(|i| ((i * 3) & 0xFF) as u8).collect();
    img[2 * 512..3 * 512].copy_from_slice(&pattern);
    fs::write(&path, &img).expect("write seed image");

    let spec = format!("sdcard@cs=2?file={}", path.display());
    let dev = build_spi_device(&spec).expect("build_spi_device should accept the spec");

    let mut core = EmulatorCore::new();
    core.attach_spi_device_shared(dev.clone());

    // Drive the CMD17 sequence through the SpiDevice trait — same
    // byte-by-byte API the bus state machine uses.
    let mut g = dev.lock().expect("device lock");
    let _ = g.on_select();
    let cmd = [0x51u8, 0x00, 0x00, 0x00, 0x02, 0xFF];
    let mut last = 0xFF;
    for &b in &cmd {
        last = g.on_byte(b);
    }
    assert_eq!(last, 0x00, "CMD17 R1 should be 0x00 (success)");

    // Drain until the 0xFE data token.
    let mut saw_token = false;
    for _ in 0..32 {
        if g.on_byte(0xFF) == 0xFE {
            saw_token = true;
            break;
        }
    }
    assert!(saw_token, "expected 0xFE data token within 32 wait clocks");

    let data: Vec<u8> = (0..512).map(|_| g.on_byte(0xFF)).collect();
    assert_eq!(data, pattern, "CMD17 must stream the seeded sector contents");

    drop(g);
    let _ = fs::remove_file(&path);
}

/// W25Q32 attached via the registry spec must round-trip a Page Program
/// → Read Data cycle and persist the new bytes to the host file. Same
/// shape as the sdcard integration test; covers the registry path the
/// CLI flag would use once it exists.
#[test]
fn cli_w25q32_attach_program_read_round_trip() {
    use cor24_emulator::peripherals::spi::build_spi_device;
    use cor24_emulator::peripherals::spi::devices::w25q32::IMAGE_SIZE;
    use std::fs;

    let path = std::env::temp_dir().join(format!(
        "cor24-w25q32-integration-{}.bin",
        std::process::id(),
    ));
    fs::write(&path, vec![0xFFu8; IMAGE_SIZE]).expect("seed erased image");

    let spec = format!("w25q32@cs=3?file={}", path.display());
    let dev = build_spi_device(&spec).expect("build_spi_device should accept the spec");

    let mut core = EmulatorCore::new();
    core.attach_spi_device_shared(dev.clone());

    let mut g = dev.lock().expect("device lock");

    // 0x06 Write Enable.
    let _ = g.on_select();
    let _ = g.on_byte(0x06);
    g.on_deselect();

    // 0x02 Page Program at addr 0x1000 with 16 bytes 0..15.
    let _ = g.on_select();
    let _ = g.on_byte(0x02);
    let _ = g.on_byte(0x00);
    let _ = g.on_byte(0x10);
    let _ = g.on_byte(0x00);
    for i in 0..16u8 {
        let _ = g.on_byte(i);
    }
    g.on_deselect();

    // Read back the same range via 0x03.
    let _ = g.on_select();
    let _ = g.on_byte(0x03);
    let _ = g.on_byte(0x00);
    let _ = g.on_byte(0x10);
    let first = g.on_byte(0x00); // MISO for the next exchange = image[addr]
    let mut readback = vec![first];
    for _ in 1..16u8 {
        readback.push(g.on_byte(0xFF));
    }
    g.on_deselect();

    let expected: Vec<u8> = (0..16u8).collect();
    assert_eq!(readback, expected);

    // Verify the host file reflects the program.
    let on_disk = fs::read(&path).expect("read back image");
    assert_eq!(&on_disk[0x1000..0x1010], expected.as_slice());

    drop(g);
    let _ = fs::remove_file(&path);
}
