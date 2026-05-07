//! API contract for the future Web UI demo.
//!
//! This crate stays WASM-target-agnostic per plan §6.1 — the actual
//! WASM/Yew (or other framework) demo lives in a separate downstream
//! repo that takes a `cargo` dependency on `cor24-emulator`. This
//! example pins the *surface* that downstream is allowed to call: any
//! API change that breaks this file breaks the Web UI before the
//! downstream notices.
//!
//! Run with `cargo run --example web_surface_smoke`. Asserted by the
//! workspace smoke test in tests/web_surface.rs.

use cor24_emulator::peripherals::i2c::{
    Add1Device, Tmp101Device, Tmp101HandleExt, Tmp101Resolution,
};
use cor24_emulator::EmulatorCore;

/// Run the full surface and return a one-line status. Anything the Web
/// UI is expected to call appears here at least once.
pub fn run_surface() -> String {
    let mut emu = EmulatorCore::new();

    // ─── Attach two devices: one universal test slave + one chip ──
    let add1 = emu
        .attach_i2c_device(Add1Device::new(0x50, 0x100))
        .expect("attach add1");
    let tmp = emu
        .attach_i2c_device(Tmp101Device::new(0x4A))
        .expect("attach tmp101");

    // ─── Chip-specific mutation through the typed handle. The Web UI's
    //     temperature slider drives this. ─────────────────────────────
    tmp.set_temperature(23.5);
    tmp.set_resolution(Tmp101Resolution::Bits10);
    assert!((tmp.temperature() - 23.5).abs() < 0.01);

    // ─── Generic mutation through `with` (other chips' analogue). ──
    add1.with(|d| d.poke(0x10));

    // ─── Move a device to a new address at runtime. ───────────────
    let _moved_to = {
        add1.set_address(0x42).expect("rerouting add1 0x50 → 0x42");
        add1.address()
    };
    assert_eq!(add1.address(), 0x42);

    // ─── Drive a tiny synthetic transaction so the I2C log has data
    //     to render — direct MMIO writes; the real Web UI runs guest
    //     code instead. ──────────────────────────────────────────────
    drive_one_byte_to(&mut emu, 0x42);

    // ─── Read the logs the way the Web UI's panels would. ────────
    let i2c_log = emu.format_i2c_log();
    let uart_log = emu.format_uart_log();
    assert!(i2c_log.contains("START"), "I2C log missing START:\n{i2c_log}");
    assert!(
        i2c_log.contains("ADDR 0x42 WR"),
        "I2C log missing addr 0x42:\n{i2c_log}"
    );
    let _ = uart_log; // empty for this synthetic run; just confirm callable

    // ─── Snapshot for the Web UI's register/memory panels. ─────────
    let snap = emu.snapshot();
    assert_eq!(snap.button & 1, 1, "button starts released (active-low)");

    // ─── Detach: clears bus routing but leaves handles alive so they
    //     can still mutate the device (last-writer-wins on re-attach). ─
    emu.detach_i2c_devices();
    add1.with(|d| d.poke(0x99));
    tmp.set_temperature(0.0);

    format!(
        "ok: i2c log {} entries; uart log {} entries; pc 0x{:06X}",
        emu.i2c_log().entries().len(),
        emu.uart_log().entries().len(),
        snap.pc,
    )
}

const IO_I2C_SCL: u32 = 0xFF0020;
const IO_I2C_SDA: u32 = 0xFF0021;

/// Hand-clock a START + addr-write + STOP so the log gets something
/// recognisable. The real Web UI just runs guest code.
fn drive_one_byte_to(emu: &mut EmulatorCore, addr7: u8) {
    // START
    emu.write_byte(IO_I2C_SDA, 1);
    emu.write_byte(IO_I2C_SCL, 1);
    emu.write_byte(IO_I2C_SDA, 0);
    emu.write_byte(IO_I2C_SCL, 0);

    // 8 bits of (addr7 << 1) — write direction
    let byte = addr7 << 1;
    for i in (0..8).rev() {
        emu.write_byte(IO_I2C_SDA, (byte >> i) & 1);
        emu.write_byte(IO_I2C_SCL, 1);
        emu.write_byte(IO_I2C_SCL, 0);
    }
    // ACK clock — master releases SDA, samples
    emu.write_byte(IO_I2C_SDA, 1);
    emu.write_byte(IO_I2C_SCL, 1);
    let _ = emu.read_byte(IO_I2C_SDA);
    emu.write_byte(IO_I2C_SCL, 0);

    // STOP
    emu.write_byte(IO_I2C_SDA, 0);
    emu.write_byte(IO_I2C_SCL, 1);
    emu.write_byte(IO_I2C_SDA, 1);
}

#[allow(dead_code)] // Used as `cargo run --example web_surface_smoke`;
                    // when included as a test module the binary entry
                    // point is unused.
fn main() {
    let status = run_surface();
    println!("{status}");
}
