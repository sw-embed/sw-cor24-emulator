//! `add1` — universal +1 test slave.
//!
//! Smallest device that exercises every I2C-bus path the state machine
//! cares about: it ACKs writes, it ACKs the addressing byte, and it
//! drives a deterministic stream of read bytes (last-written + 1, +2,
//! ... mod `wrap`). Every other device — TMP101, EEPROM, DS3231 — is a
//! more elaborate version of the same shape.

use crate::peripherals::i2c::device::{Ack, I2cDevice};

/// Default wrap modulus: 256 (8-bit wrap).
pub const DEFAULT_WRAP: u16 = 0x100;

/// Universal "+1" test slave. Stores `last`; on every read returns
/// `last = (last + 1) % wrap`.
pub struct Add1Device {
    address: u8,
    last: u8,
    wrap: u16,
}

impl Add1Device {
    /// Construct an Add1 device at the given 7-bit address. `wrap` is
    /// the modulus for the read sequence (default 256). `wrap = 0` is
    /// promoted to `DEFAULT_WRAP` to avoid a divide-by-zero.
    pub fn new(address: u8, wrap: u16) -> Self {
        let wrap = if wrap == 0 { DEFAULT_WRAP } else { wrap };
        Self {
            address: address & 0x7F,
            last: 0,
            wrap,
        }
    }

    /// Inspect the stored byte without advancing the read counter.
    pub fn peek(&self) -> u8 {
        self.last
    }

    /// Force the stored byte. The next bus read will return
    /// `(value + 1) % wrap`.
    pub fn poke(&mut self, value: u8) {
        self.last = value;
    }

    pub fn wrap(&self) -> u16 {
        self.wrap
    }
}

impl I2cDevice for Add1Device {
    fn address(&self) -> u8 {
        self.address
    }

    fn set_address(&mut self, addr: u8) {
        self.address = addr & 0x7F;
    }

    fn name(&self) -> &str {
        "add1"
    }

    fn on_write_byte(&mut self, byte: u8) -> Ack {
        self.last = byte;
        Ack::Ack
    }

    fn on_read_byte(&mut self) -> u8 {
        self.last = ((self.last as u16 + 1) % self.wrap) as u8;
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_default_address() {
        let d = Add1Device::new(0x50, 0);
        assert_eq!(d.name(), "add1");
        assert_eq!(d.address(), 0x50);
        assert_eq!(d.wrap(), DEFAULT_WRAP);
    }

    #[test]
    fn write_then_read_increments() {
        let mut d = Add1Device::new(0x50, 0x100);
        assert_eq!(d.on_write_byte(0x42), Ack::Ack);
        assert_eq!(d.on_read_byte(), 0x43);
        assert_eq!(d.on_read_byte(), 0x44);
        assert_eq!(d.on_read_byte(), 0x45);
    }

    #[test]
    fn wrap_at_modulus() {
        let mut d = Add1Device::new(0x50, 10);
        d.on_write_byte(9);
        assert_eq!(d.on_read_byte(), 0);
        assert_eq!(d.on_read_byte(), 1);
    }

    #[test]
    fn wrap_at_256_default() {
        let mut d = Add1Device::new(0x50, 0x100);
        d.on_write_byte(0xFF);
        assert_eq!(d.on_read_byte(), 0x00);
        assert_eq!(d.on_read_byte(), 0x01);
    }

    #[test]
    fn poke_overrides_state() {
        let mut d = Add1Device::new(0x50, 0x100);
        d.on_write_byte(0x10);
        d.poke(0x20);
        assert_eq!(d.peek(), 0x20);
        assert_eq!(d.on_read_byte(), 0x21);
    }

    #[test]
    fn set_address_updates_responding_address() {
        let mut d = Add1Device::new(0x50, 0x100);
        assert_eq!(d.address(), 0x50);
        d.set_address(0x42);
        assert_eq!(d.address(), 0x42);
        // High bit always cleared
        d.set_address(0xFF);
        assert_eq!(d.address(), 0x7F);
    }

    #[test]
    fn zero_wrap_promotes_to_default() {
        let d = Add1Device::new(0x50, 0);
        assert_eq!(d.wrap(), DEFAULT_WRAP);
    }
}
