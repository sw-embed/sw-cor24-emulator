//! `tmp101` — TI TMP101 temperature sensor (I2C).
//!
//! Models the four chip registers the imported `tmp101.lgo` demo
//! cares about:
//!   - `0x00` — Temperature (16-bit, read-only).
//!   - `0x01` — Configuration (8-bit, read/write).
//!   - `0x02` — T_LOW   (16-bit, read/write — stub: stored verbatim).
//!   - `0x03` — T_HIGH  (16-bit, read/write — stub: stored verbatim).
//!
//! The 12-bit temperature value is laid out in the upper 12 bits of
//! the 16-bit register (0.0625°C / LSB; bits[3:0] always zero). The
//! current resolution mode in the config register is honoured at read
//! time, so a guest that reconfigures resolution between reads sees
//! values quantized to the new step size.

use crate::peripherals::i2c::device::{Ack, I2cDevice};

/// Default 7-bit address for the COR24-TB layout.
pub const DEFAULT_ADDRESS: u8 = 0x4A;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tmp101Resolution {
    Bits9,
    Bits10,
    Bits11,
    Bits12,
}

impl Tmp101Resolution {
    fn from_config(config: u8) -> Self {
        match (config >> 5) & 0x03 {
            0b00 => Self::Bits9,
            0b01 => Self::Bits10,
            0b10 => Self::Bits11,
            _ => Self::Bits12,
        }
    }

    fn mask(self) -> u16 {
        // Mask applied to the 12-bit temperature value before it is
        // shifted into the 16-bit register layout.
        match self {
            Self::Bits9 => 0x0FF8,
            Self::Bits10 => 0x0FFC,
            Self::Bits11 => 0x0FFE,
            Self::Bits12 => 0x0FFF,
        }
    }
}

pub struct Tmp101Device {
    address: u8,
    pointer: u8,
    config: u8,
    /// Configured temperature in °C. Quantized at read time using the
    /// current config resolution, so set-then-reconfigure works.
    temperature: f32,
    /// Datasheet defaults: 75°C / 80°C in 12-bit register layout.
    t_low: u16,
    t_high: u16,
    /// Byte index within the current write transaction. 0 = next byte
    /// is the pointer; 1+ = data bytes for the selected register.
    write_idx: u8,
    /// Byte index within the current read transaction.
    read_idx: u8,
}

impl Tmp101Device {
    pub fn new(address: u8) -> Self {
        Self {
            address: address & 0x7F,
            pointer: 0,
            config: 0,
            temperature: 0.0,
            t_low: 0x4B00,
            t_high: 0x5000,
            write_idx: 0,
            read_idx: 0,
        }
    }

    /// Set the configured temperature in °C. Quantized at read time
    /// based on the current resolution mode.
    pub fn set_temperature(&mut self, celsius: f32) {
        self.temperature = celsius;
    }

    /// Currently-configured temperature in °C (the value the guest
    /// would read back, given the current resolution).
    pub fn temperature(&self) -> f32 {
        let val = self.temp_register_value();
        // Sign-extend the 12-bit value to i32, scale by 0.0625°C/LSB.
        let signed = sign_extend_12(val);
        signed as f32 * 0.0625
    }

    /// Set the configuration register directly. Useful in tests.
    pub fn set_config(&mut self, config: u8) {
        self.config = config;
    }

    pub fn config(&self) -> u8 {
        self.config
    }

    /// Set the resolution by writing the appropriate bits to config.
    /// Other config bits are preserved.
    pub fn set_resolution(&mut self, res: Tmp101Resolution) {
        let bits: u8 = match res {
            Tmp101Resolution::Bits9 => 0b00,
            Tmp101Resolution::Bits10 => 0b01,
            Tmp101Resolution::Bits11 => 0b10,
            Tmp101Resolution::Bits12 => 0b11,
        };
        self.config = (self.config & !0x60) | (bits << 5);
    }

    pub fn resolution(&self) -> Tmp101Resolution {
        Tmp101Resolution::from_config(self.config)
    }

    /// 12-bit temperature value as the chip would currently report it,
    /// quantized to the active resolution.
    fn temp_register_value(&self) -> u16 {
        let raw = (self.temperature / 0.0625).round() as i32;
        let clamped = raw.clamp(-2048, 2047);
        let val_12bit = (clamped & 0x0FFF) as u16;
        val_12bit & self.resolution().mask()
    }

    /// 16-bit temperature register (12-bit value in the upper 12 bits;
    /// lower 4 bits always 0).
    fn temp_register(&self) -> u16 {
        self.temp_register_value() << 4
    }

    fn read_register_byte(&self, register: u8, idx: u8) -> u8 {
        match register {
            0x00 => byte_of_u16(self.temp_register(), idx),
            0x01 => self.config,
            0x02 => byte_of_u16(self.t_low, idx),
            0x03 => byte_of_u16(self.t_high, idx),
            _ => 0xFF,
        }
    }

    fn write_register_byte(&mut self, register: u8, idx: u8, byte: u8) {
        match register {
            0x00 => { /* temp register is read-only */ }
            0x01 => self.config = byte,
            0x02 => self.t_low = patch_u16(self.t_low, idx, byte),
            0x03 => self.t_high = patch_u16(self.t_high, idx, byte),
            _ => {}
        }
    }
}

fn sign_extend_12(value: u16) -> i32 {
    let v = (value & 0x0FFF) as i32;
    if v & 0x0800 != 0 { v - 0x1000 } else { v }
}

fn byte_of_u16(reg: u16, idx: u8) -> u8 {
    // Even index = high byte; odd = low byte. 16-bit reads on TMP101
    // continue past the second byte by re-reading from the start.
    if idx.is_multiple_of(2) {
        (reg >> 8) as u8
    } else {
        (reg & 0xFF) as u8
    }
}

fn patch_u16(reg: u16, idx: u8, byte: u8) -> u16 {
    if idx == 1 {
        (reg & 0x00FF) | ((byte as u16) << 8)
    } else if idx == 2 {
        (reg & 0xFF00) | (byte as u16)
    } else {
        reg
    }
}

impl I2cDevice for Tmp101Device {
    fn address(&self) -> u8 {
        self.address
    }

    fn set_address(&mut self, addr: u8) {
        self.address = addr & 0x7F;
    }

    fn name(&self) -> &str {
        "tmp101"
    }

    fn on_start(&mut self) {
        self.write_idx = 0;
        self.read_idx = 0;
    }

    fn on_write_byte(&mut self, byte: u8) -> Ack {
        if self.write_idx == 0 {
            // Pointer byte: low 2 bits select the register.
            self.pointer = byte & 0x03;
        } else {
            self.write_register_byte(self.pointer, self.write_idx, byte);
        }
        self.write_idx = self.write_idx.saturating_add(1);
        Ack::Ack
    }

    fn on_read_byte(&mut self) -> u8 {
        let byte = self.read_register_byte(self.pointer, self.read_idx);
        self.read_idx = self.read_idx.wrapping_add(1);
        byte
    }
}

/// Ergonomic extension on `I2cHandle<Tmp101Device>` so callers don't
/// need to spell out `handle.with(|d| d.set_temperature(c))`.
pub trait Tmp101HandleExt {
    fn set_temperature(&self, celsius: f32);
    fn temperature(&self) -> f32;
    fn set_resolution(&self, res: Tmp101Resolution);
    fn resolution(&self) -> Tmp101Resolution;
}

impl Tmp101HandleExt for crate::peripherals::i2c::I2cHandle<Tmp101Device> {
    fn set_temperature(&self, celsius: f32) {
        self.with(|d| d.set_temperature(celsius));
    }
    fn temperature(&self) -> f32 {
        self.with(|d| d.temperature())
    }
    fn set_resolution(&self, res: Tmp101Resolution) {
        self.with(|d| d.set_resolution(res));
    }
    fn resolution(&self) -> Tmp101Resolution {
        self.with(|d| d.resolution())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_temp_register(d: &mut Tmp101Device) -> (u8, u8) {
        d.on_start();
        // pretend the master wrote pointer=0 then issued repeated START
        // for a read; for the tests we shortcut by setting pointer.
        d.pointer = 0;
        let h = d.on_read_byte();
        let l = d.on_read_byte();
        (h, l)
    }

    #[test]
    fn name_and_default_address() {
        let d = Tmp101Device::new(DEFAULT_ADDRESS);
        assert_eq!(d.name(), "tmp101");
        assert_eq!(d.address(), 0x4A);
    }

    #[test]
    fn temp_register_zero_celsius() {
        let mut d = Tmp101Device::new(0x4A);
        d.set_temperature(0.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x00, 0x00));
    }

    #[test]
    fn temp_register_25_celsius_12bit() {
        // 12-bit mode: 25.0 / 0.0625 = 400 = 0x190; register = 0x1900.
        let mut d = Tmp101Device::new(0x4A);
        d.set_resolution(Tmp101Resolution::Bits12);
        d.set_temperature(25.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x19, 0x00));
    }

    #[test]
    fn temp_register_25_celsius_10bit_demo_path() {
        // 10-bit mode (R1R0=0b01, config=0x20): 25.0 → 100 (0.25°C/LSB),
        // 16-bit register = 0x1900. Demo decode: ((0x19<<2)|(0x00>>6))
        // sign-extended from 10-bit = 100 = 25.0°C. Matches the
        // tmp101.lgo path exactly.
        let mut d = Tmp101Device::new(0x4A);
        d.set_config(0x20);
        d.set_temperature(25.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x19, 0x00));

        let demo_decoded = sign_extend_10(((h as u16) << 2) | ((l as u16) >> 6));
        assert_eq!(demo_decoded, 100);
    }

    #[test]
    fn temp_register_negative_25_celsius() {
        // -25.0 / 0.0625 = -400. As 12-bit signed: 0xE70. Register: 0xE700.
        let mut d = Tmp101Device::new(0x4A);
        d.set_temperature(-25.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0xE7, 0x00));

        let demo_decoded = sign_extend_10(((h as u16) << 2) | ((l as u16) >> 6));
        assert_eq!(demo_decoded, -100); // -25.0 * 4
    }

    #[test]
    fn temp_register_max_positive_12bit() {
        // 127.9375°C = 0x7FF in 12-bit. Register: 0x7FF0.
        let mut d = Tmp101Device::new(0x4A);
        d.set_resolution(Tmp101Resolution::Bits12);
        d.set_temperature(127.9375);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x7F, 0xF0));
    }

    #[test]
    fn temp_register_clamps_to_chip_range() {
        // Out-of-range temperatures clamp to the 12-bit signed range.
        let mut d = Tmp101Device::new(0x4A);
        d.set_resolution(Tmp101Resolution::Bits12);
        d.set_temperature(1000.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x7F, 0xF0));

        d.set_temperature(-1000.0);
        let (h, l) = read_temp_register(&mut d);
        assert_eq!((h, l), (0x80, 0x00));
    }

    #[test]
    fn config_register_round_trip() {
        let mut d = Tmp101Device::new(0x4A);
        d.on_start();
        // Pointer = 0x01 (config), data = 0x60 (12-bit mode bits).
        assert_eq!(d.on_write_byte(0x01), Ack::Ack);
        assert_eq!(d.on_write_byte(0x60), Ack::Ack);
        assert_eq!(d.config(), 0x60);
        assert_eq!(d.resolution(), Tmp101Resolution::Bits12);

        // Read it back.
        d.on_start();
        d.pointer = 0x01;
        assert_eq!(d.on_read_byte(), 0x60);
    }

    #[test]
    fn pointer_set_persists_across_starts() {
        let mut d = Tmp101Device::new(0x4A);
        d.set_temperature(25.0);

        // Setup transaction: pointer = 0x00 (temp).
        d.on_start();
        assert_eq!(d.on_write_byte(0x00), Ack::Ack);

        // Read transaction (no further pointer write): high then low.
        d.on_start();
        let h = d.on_read_byte();
        let l = d.on_read_byte();
        assert_eq!((h, l), (0x19, 0x00));
    }

    #[test]
    fn read_idx_wraps_for_continuous_read() {
        // libi2c always sends master-ACK after each read byte, so the
        // bus may pull a third byte that the master then discards.
        // The device must not panic and must stay coherent.
        let mut d = Tmp101Device::new(0x4A);
        d.set_temperature(25.0);
        d.on_start();
        let h1 = d.on_read_byte();
        let l1 = d.on_read_byte();
        let h2 = d.on_read_byte();
        let l2 = d.on_read_byte();
        assert_eq!(h1, h2);
        assert_eq!(l1, l2);
    }

    #[test]
    fn set_resolution_does_not_lose_other_config_bits() {
        let mut d = Tmp101Device::new(0x4A);
        d.set_config(0x05); // bits 2 and 0 set; resolution = 9-bit
        d.set_resolution(Tmp101Resolution::Bits12);
        assert_eq!(d.config() & 0x60, 0x60); // R1R0 = 11
        assert_eq!(d.config() & !0x60, 0x05); // other bits preserved
    }
}

// Re-implementation of the demo's 10-bit sign extension, used by the
// unit tests above to mirror the guest-side decode.
#[cfg(test)]
fn sign_extend_10(value: u16) -> i32 {
    let v = (value & 0x03FF) as i32;
    if v & 0x0200 != 0 { v - 0x0400 } else { v }
}
