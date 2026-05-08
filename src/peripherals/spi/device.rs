//! SPI slave-device trait.
//!
//! Simpler than I2C — no addressing, no ACK/NAK, no open-drain. The
//! device sees `on_select` (SELN falling edge), then a stream of
//! `on_byte(mosi)` calls that each return the byte the slave will drive
//! on MISO during the *next* exchange (one-byte echo delay, matching
//! how a real shift register actually presents data: the slave needed
//! to know the byte to send before the master clocked bit 0). On
//! `on_select` the device returns its first MISO byte (pre-loaded into
//! the bus shift_out before the first SCLK rise).
//!
//! Mode 0 only (CPOL=0, CPHA=0), MSB-first. Multi-mode support is
//! plan §9 future work.

pub trait SpiDevice: Send + 'static {
    fn name(&self) -> &str {
        "spi-device"
    }

    /// Master pulled SELN low — bus is selecting this slave. Return the
    /// first byte the slave wants on MISO during the *next* 8-clock
    /// exchange (the bus pre-loads its shift_out from this).
    fn on_select(&mut self) -> u8 {
        0
    }

    /// One full 8-bit exchange completed. `mosi` is what the master
    /// just clocked in. Returns the byte the slave will drive on MISO
    /// during the *next* 8-clock exchange.
    fn on_byte(&mut self, _mosi: u8) -> u8 {
        0
    }

    /// Master released SELN. Mid-byte state in the bus is reset by the
    /// caller; this is for device-side bookkeeping (commit-on-deselect
    /// semantics, etc.).
    fn on_deselect(&mut self) {}

    /// Optional time-tick. Called once per CPU instruction. Use sparingly
    /// — only for devices that model real time (conversion delays etc.).
    fn on_tick(&mut self) {}
}
