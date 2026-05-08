//! SPI bus state machine.
//!
//! Much simpler than I2C — no addressing, no START/STOP, no
//! open-drain wired-AND. Just a shift register clocked by SCLK while
//! SELN is low, mode 0 (CPOL=0, CPHA=0): master sets MOSI on the
//! falling edge, slave samples on the rising edge.
//!
//! State accumulates the master's MOSI byte (MSB-first) and exposes
//! the slave-driven MISO byte for the same 8-bit exchange. With no
//! `SpiDevice` attached yet (Phase C.4), `shift_out` stays 0 and the
//! master always reads 0x00 back.
//!
//! `step()` is called from `state.rs::write_io` after every SPI MMIO
//! write, so the bus advances on SCLK / SELN edges *and* on MOSI bit
//! updates (the latter is just a level update, never a transition).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpiBusState {
    /// Last SCLK level seen (used for rising/falling edge detection).
    pub last_sclk: bool,
    /// Last SELN level seen (active-low; used for deselect-edge reset).
    pub last_seln: bool,
    /// Bits accumulated in the current MOSI byte, MSB-first. Cleared
    /// when a byte completes or when SELN rises mid-byte.
    pub shift_in: u8,
    /// Bits the slave is driving on MISO, MSB-first. Stays 0 until a
    /// `SpiDevice` is attached and pre-loads it on byte boundaries.
    pub shift_out: u8,
    /// Bits collected in the current byte (0..=7 mid-byte; resets to
    /// 0 on byte completion or SELN rise).
    pub bit_count: u8,
    /// Most recent fully-shifted MOSI byte.
    pub last_mosi_byte: Option<u8>,
    /// Most recent fully-shifted MISO byte (what the slave drove
    /// during the same 8-clock exchange).
    pub last_miso_byte: Option<u8>,
    /// Cumulative byte-exchange count — useful for smoke tests that
    /// want to confirm the bus made progress.
    pub bytes_exchanged: u32,
}

impl SpiBusState {
    pub fn new() -> Self {
        Self {
            last_sclk: false,
            last_seln: true, // active-low: idle = deselected
            shift_in: 0,
            shift_out: 0,
            bit_count: 0,
            last_mosi_byte: None,
            last_miso_byte: None,
            bytes_exchanged: 0,
        }
    }

    /// Advance the state machine after the master writes any SPI MMIO
    /// register. `sclk` / `seln` are the current driven levels;
    /// `mosi_bit` is the bit currently on MOSI (master writes are
    /// effectively single-bit at bit 0 of `IO_SPI_DATA` in the
    /// canonical spixchg loop, but any byte the guest wrote is
    /// accepted — we only sample its bit 0). `_instruction` is
    /// reserved for the SPI log (Phase C.6+); unused today.
    pub fn step(&mut self, sclk: bool, mosi_bit: bool, seln: bool, _instruction: u64) {
        let prev_sclk = self.last_sclk;
        let prev_seln = self.last_seln;

        // SELN rising edge while not idle: abort mid-byte (matches real
        // chips that drop the shift state on CS rise).
        if seln && !prev_seln && self.bit_count != 0 {
            self.shift_in = 0;
            self.bit_count = 0;
            self.shift_out = 0;
        }

        // SCLK rising edge with slave selected: sample MOSI bit, shift
        // it in MSB-first. Shift the MISO register left by 1 too so
        // the next bit out is at the MSB position.
        if !seln && sclk && !prev_sclk {
            self.shift_in = (self.shift_in << 1) | (mosi_bit as u8);
            self.shift_out <<= 1;
            self.bit_count += 1;
            if self.bit_count == 8 {
                self.last_mosi_byte = Some(self.shift_in);
                self.last_miso_byte = Some(self.shift_in_to_miso_byte());
                self.bytes_exchanged = self.bytes_exchanged.saturating_add(1);
                self.bit_count = 0;
                self.shift_in = 0;
                self.shift_out = 0;
            }
        }

        self.last_sclk = sclk;
        self.last_seln = seln;
    }

    /// Reconstruct the byte the slave drove during the just-finished
    /// 8-clock exchange. With shift_out shifted left 8 times during
    /// the exchange (on each rising edge) the original byte is
    /// recoverable — but for a byte-boundary observability snapshot
    /// the simplest correct value is "0 until we have a slave"; the
    /// SpiDevice integration in Phase C.5 reloads shift_out at byte
    /// boundaries and tracks the pre-shift value separately.
    fn shift_in_to_miso_byte(&self) -> u8 {
        // No SpiDevice yet — slave drove all-zeros. Phase C.5 replaces
        // this with the byte the device returned from on_byte().
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive one bit by setting MOSI level, then pulsing SCLK 0->1->0
    /// while SELN stays at the supplied level.
    fn clock_bit(bus: &mut SpiBusState, mosi: bool, seln: bool) {
        bus.step(false, mosi, seln, 0); // ensure SCLK low
        bus.step(true, mosi, seln, 0); // rising edge → sample
        bus.step(false, mosi, seln, 0); // falling edge
    }

    #[test]
    fn idle_after_new() {
        let bus = SpiBusState::new();
        assert_eq!(bus.bit_count, 0);
        assert_eq!(bus.bytes_exchanged, 0);
        assert!(bus.last_seln, "SELN starts high (deselected)");
        assert_eq!(bus.last_mosi_byte, None);
    }

    #[test]
    fn exchange_one_byte_msb_first() {
        let mut bus = SpiBusState::new();
        // Select the slave.
        bus.step(false, false, false, 0);
        // Send 0xA5 = 1010 0101 MSB-first.
        for bit in [true, false, true, false, false, true, false, true] {
            clock_bit(&mut bus, bit, false);
        }
        assert_eq!(bus.last_mosi_byte, Some(0xA5));
        assert_eq!(bus.last_miso_byte, Some(0x00)); // no device yet
        assert_eq!(bus.bytes_exchanged, 1);
        assert_eq!(bus.bit_count, 0);
    }

    #[test]
    fn sclk_with_seln_high_does_nothing() {
        let mut bus = SpiBusState::new();
        // SELN stays high (1 = deselected) the whole time.
        for _ in 0..8 {
            clock_bit(&mut bus, true, true);
        }
        assert_eq!(bus.bit_count, 0);
        assert_eq!(bus.bytes_exchanged, 0);
        assert_eq!(bus.last_mosi_byte, None);
    }

    #[test]
    fn mid_byte_deselect_resets() {
        let mut bus = SpiBusState::new();
        bus.step(false, false, false, 0); // select
        // Clock 4 bits.
        for bit in [true, true, false, true] {
            clock_bit(&mut bus, bit, false);
        }
        assert_eq!(bus.bit_count, 4);
        // Deselect → reset.
        bus.step(false, false, true, 0);
        assert_eq!(bus.bit_count, 0);
        assert_eq!(bus.shift_in, 0);
        // Reselect and clock a fresh byte.
        bus.step(false, false, false, 0);
        for bit in [false, false, false, false, false, false, false, true] {
            clock_bit(&mut bus, bit, false);
        }
        assert_eq!(bus.last_mosi_byte, Some(0x01));
        assert_eq!(bus.bytes_exchanged, 1);
    }

    #[test]
    fn bytes_exchanged_increments_per_byte() {
        let mut bus = SpiBusState::new();
        bus.step(false, false, false, 0);
        // 3 consecutive bytes, all 0xFF.
        for _ in 0..3 {
            for _ in 0..8 {
                clock_bit(&mut bus, true, false);
            }
        }
        assert_eq!(bus.bytes_exchanged, 3);
        assert_eq!(bus.last_mosi_byte, Some(0xFF));
    }

    #[test]
    fn sampling_happens_on_rising_edge_only() {
        let mut bus = SpiBusState::new();
        bus.step(false, false, false, 0);
        // Set MOSI low, pulse: shift_in stays 0.
        clock_bit(&mut bus, false, false);
        assert_eq!(bus.shift_in, 0);
        // Toggle MOSI to 1 BEFORE the next rise; expect that bit
        // captured (not the previous level).
        bus.step(false, true, false, 0); // setup MOSI on falling
        bus.step(true, true, false, 0); // rising edge
        assert_eq!(bus.shift_in, 0b01);
        bus.step(false, true, false, 0); // back low
        assert_eq!(bus.bit_count, 2);
    }
}
