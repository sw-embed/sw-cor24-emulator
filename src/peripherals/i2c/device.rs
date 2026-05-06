//! I2C slave-device trait.
//!
//! A device receives bus events (start, byte writes, byte reads, stop)
//! and decides whether to ACK and what bytes to drive on reads. `address`
//! and `set_address` are mandatory: the 7-bit bus address is a runtime
//! board-level concern (DIP switches, accidental conflicts), not a
//! compile-time chip property — the Web UI moves devices around at
//! runtime through `I2cHandle::set_address`.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Ack {
    Ack,
    #[default]
    Nak,
}

pub trait I2cDevice: Send + 'static {
    fn address(&self) -> u8;
    fn set_address(&mut self, addr: u8);

    fn name(&self) -> &str {
        "i2c-device"
    }

    fn on_start(&mut self) {}
    fn on_write_byte(&mut self, _byte: u8) -> Ack {
        Ack::Nak
    }
    fn on_read_byte(&mut self) -> u8 {
        0xFF
    }
    fn on_master_ack(&mut self) {}
    fn on_master_nak(&mut self) {}
    fn on_stop(&mut self) {}
    fn on_tick(&mut self) {}
    fn stretching_scl(&self) -> bool {
        false
    }
}
