//! `echo` — universal SPI test slave.
//!
//! Smallest device that exercises the bus's per-byte dispatch end to
//! end. Stores a single-byte buffer holding "what to drive on MISO
//! during the next exchange". On every byte the master clocks in,
//! the buffer is replaced with that MOSI byte; the next exchange then
//! drives it back out. Net effect: MISO byte N+1 == MOSI byte N — the
//! one-byte delay a real hardware shift register exhibits, since the
//! slave needs the byte loaded before the master clocks bit 0.
//!
//! On `on_select` the device returns its current buffer (the seed for
//! the very first exchange after CS goes low).

use crate::peripherals::spi::device::SpiDevice;

pub struct EchoDevice {
    /// Byte the slave will drive on MISO during the next 8-clock
    /// exchange. Updated by `on_byte` (latches MOSI for next time) and
    /// by `poke` (test/UI control).
    buffer: u8,
}

impl EchoDevice {
    /// Construct an EchoDevice with the given seed byte. The seed is
    /// what the slave drives on MISO during the *first* exchange after
    /// `on_select`.
    pub fn new(seed: u8) -> Self {
        Self { buffer: seed }
    }

    /// Inspect the buffered byte without touching it.
    pub fn peek(&self) -> u8 {
        self.buffer
    }

    /// Force the buffered byte. The next exchange will return this
    /// value on MISO.
    pub fn poke(&mut self, value: u8) {
        self.buffer = value;
    }
}

impl SpiDevice for EchoDevice {
    fn name(&self) -> &str {
        "echo"
    }

    fn on_select(&mut self) -> u8 {
        self.buffer
    }

    fn on_byte(&mut self, mosi: u8) -> u8 {
        // Latch the just-clocked MOSI byte; the bus then drives it out
        // on the *next* exchange — a one-byte echo delay.
        self.buffer = mosi;
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_default() {
        let d = EchoDevice::new(0x00);
        assert_eq!(d.name(), "echo");
    }

    #[test]
    fn on_select_returns_seed() {
        let mut d = EchoDevice::new(0xA5);
        assert_eq!(d.on_select(), 0xA5);
    }

    #[test]
    fn on_byte_latches_mosi_for_next_exchange() {
        // The trait contract is: on_byte returns the byte the slave
        // will drive on MISO during the *next* exchange. EchoDevice
        // satisfies that by latching the just-clocked MOSI byte and
        // returning it — the bus then uses that as next current_miso.
        let mut d = EchoDevice::new(0x11);
        // First on_byte: latch 0x42; next exchange will drive 0x42.
        assert_eq!(d.on_byte(0x42), 0x42);
        // Second on_byte: latch 0x99; next exchange will drive 0x99.
        assert_eq!(d.on_byte(0x99), 0x99);
        // Third on_byte: latch 0x00.
        assert_eq!(d.on_byte(0x00), 0x00);
    }

    #[test]
    fn poke_peek_round_trip() {
        let mut d = EchoDevice::new(0x00);
        d.poke(0x5A);
        assert_eq!(d.peek(), 0x5A);
        // on_select reports the poked value; the next exchange's MISO
        // will drive 0x5A.
        assert_eq!(d.on_select(), 0x5A);
    }

    #[test]
    fn on_deselect_does_not_clear_buffer() {
        // Echo is stateless re: select; the buffer survives a deselect
        // cycle so re-select continues from where we left off.
        let mut d = EchoDevice::new(0x10);
        d.on_byte(0x20);
        d.on_deselect();
        assert_eq!(d.peek(), 0x20);
        assert_eq!(d.on_select(), 0x20);
    }
}
