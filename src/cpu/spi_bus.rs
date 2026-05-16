//! SPI bus state machine.
//!
//! Much simpler than I2C — no addressing, no START/STOP, no
//! open-drain wired-AND. Just a shift register clocked by SCLK while
//! SELN is low, mode 0 (CPOL=0, CPHA=0): master sets MOSI on the
//! falling edge, slave samples on the rising edge.
//!
//! The bus accumulates the master's MOSI byte (MSB-first) and tracks
//! the slave-driven byte for the same 8-bit exchange. With a
//! `SpiDevice` attached, the device's `on_select` pre-loads the byte
//! to drive on MISO during the first exchange after CS goes low; on
//! each byte completion `on_byte(mosi)` returns the byte to drive
//! during the next exchange (one-byte echo delay — the slave needed
//! to know what to send before the master clocked bit 0).
//!
//! `step()` is called from `state.rs::write_io` after every SPI MMIO
//! write, so the bus advances on SCLK / SELN edges *and* on MOSI bit
//! updates (the latter is just a level update, never a transition).

use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::peripherals::spi::device::SpiDevice;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SpiBusState {
    /// Last SCLK level seen (used for rising/falling edge detection).
    pub last_sclk: bool,
    /// Last SELN level seen (active-low; used for select/deselect-edge
    /// detection).
    pub last_seln: bool,
    /// Bits accumulated in the current MOSI byte, MSB-first. Cleared
    /// when a byte completes or when SELN rises mid-byte.
    pub shift_in: u8,
    /// Bits the slave is driving on MISO, MSB-first. Pre-loaded on
    /// SELN falling and on each byte completion from the attached
    /// `SpiDevice`. Stays 0 with no device attached.
    pub shift_out: u8,
    /// Bits collected in the current byte (0..=7 mid-byte; resets to
    /// 0 on byte completion or SELN rise).
    pub bit_count: u8,
    /// Most recent fully-shifted MOSI byte.
    pub last_mosi_byte: Option<u8>,
    /// Most recent fully-shifted MISO byte (the byte the slave drove
    /// during the same 8-clock exchange).
    pub last_miso_byte: Option<u8>,
    /// Cumulative byte-exchange count — useful for smoke tests that
    /// want to confirm the bus made progress.
    pub bytes_exchanged: u32,
    /// Most recently sampled MISO line bit. Captured on each SCLK
    /// rising edge from the MSB of `shift_out` *before* the shift —
    /// i.e., the bit the slave was driving during the high phase, the
    /// bit the master would sample. Read by the CPU through bit 0 of
    /// `IO_SPI_DATA`. Cleared on SELN rise.
    pub last_miso_bit: bool,
    /// Byte the slave is currently driving on MISO across the
    /// in-flight 8-clock exchange. Used to populate `last_miso_byte`
    /// on byte completion. Loaded from `SpiDevice::on_select` when
    /// SELN falls and from `SpiDevice::on_byte` after each completed
    /// byte.
    #[serde(skip)]
    current_miso_byte: u8,
    /// Single attached SPI slave. Plan §9 future work: multi-slave
    /// SELN bitmask plus a per-slot device slot.
    #[serde(skip, default)]
    pub device: Option<Arc<Mutex<dyn SpiDevice>>>,
}

impl fmt::Debug for SpiBusState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpiBusState")
            .field("last_sclk", &self.last_sclk)
            .field("last_seln", &self.last_seln)
            .field("shift_in", &self.shift_in)
            .field("shift_out", &self.shift_out)
            .field("bit_count", &self.bit_count)
            .field("last_mosi_byte", &self.last_mosi_byte)
            .field("last_miso_byte", &self.last_miso_byte)
            .field("bytes_exchanged", &self.bytes_exchanged)
            .field("last_miso_bit", &self.last_miso_bit)
            .field("current_miso_byte", &self.current_miso_byte)
            .field("attached", &self.device.is_some())
            .finish()
    }
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
            last_miso_bit: false,
            current_miso_byte: 0,
            device: None,
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

        // SELN falling edge: device gets selected. Pre-load the byte
        // the slave wants to drive during the first 8-clock exchange.
        // The MSB is what the slave will drive on MISO before the
        // master's first SCLK rise.
        if prev_seln && !seln {
            let first = self.with_device(|d| d.on_select()).unwrap_or(0);
            self.current_miso_byte = first;
            self.shift_out = first;
            self.last_miso_bit = (first >> 7) & 1 != 0;
        }

        // SELN rising edge: device gets deselected. Drop any partial
        // byte (matches real chips that lose mid-byte shift state on
        // CS rise) and notify the device.
        if !prev_seln && seln {
            if self.bit_count != 0 {
                self.shift_in = 0;
                self.bit_count = 0;
            }
            self.shift_out = 0;
            self.current_miso_byte = 0;
            self.last_miso_bit = false;
            self.with_device(|d| {
                d.on_deselect();
            });
        }

        // SCLK rising edge with slave selected: sample MOSI bit, shift
        // it in MSB-first. Capture the MISO bit being driven (MSB of
        // shift_out) *before* shifting — that's the bit the master
        // samples right after pulling SCLK high.
        if !seln && sclk && !prev_sclk {
            self.last_miso_bit = (self.shift_out >> 7) & 1 != 0;
            self.shift_in = (self.shift_in << 1) | (mosi_bit as u8);
            self.shift_out <<= 1;
            self.bit_count += 1;
            if self.bit_count == 8 {
                let mosi_byte = self.shift_in;
                self.last_mosi_byte = Some(mosi_byte);
                self.last_miso_byte = Some(self.current_miso_byte);
                self.bytes_exchanged = self.bytes_exchanged.saturating_add(1);
                self.bit_count = 0;
                self.shift_in = 0;
                // Latch the next MISO byte from the device for the
                // upcoming exchange. With no device, the slave drives
                // 0x00. The MSB becomes the bit the slave will drive
                // before the next SCLK rise.
                let next = self.with_device(|d| d.on_byte(mosi_byte)).unwrap_or(0);
                self.current_miso_byte = next;
                self.shift_out = next;
            }
        }

        self.last_sclk = sclk;
        self.last_seln = seln;
    }

    /// Lock the attached device (if any) and run `f` against it.
    /// Returns `None` if no device is attached or the lock is poisoned.
    fn with_device<R>(&self, f: impl FnOnce(&mut dyn SpiDevice) -> R) -> Option<R> {
        let arc = self.device.as_ref()?;
        let mut guard = arc.lock().ok()?;
        Some(f(&mut *guard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::spi::devices::echo::EchoDevice;

    /// Drive one bit by setting MOSI level, then pulsing SCLK 0->1->0
    /// while SELN stays at the supplied level.
    fn clock_bit(bus: &mut SpiBusState, mosi: bool, seln: bool) {
        bus.step(false, mosi, seln, 0); // ensure SCLK low
        bus.step(true, mosi, seln, 0); // rising edge → sample
        bus.step(false, mosi, seln, 0); // falling edge
    }

    /// Send a full byte MSB-first while SELN stays low.
    fn clock_byte(bus: &mut SpiBusState, mosi_byte: u8) {
        for i in (0..8).rev() {
            let bit = ((mosi_byte >> i) & 1) != 0;
            clock_bit(bus, bit, false);
        }
    }

    fn attach_echo(bus: &mut SpiBusState, seed: u8) {
        bus.device = Some(Arc::new(Mutex::new(EchoDevice::new(seed))));
    }

    #[test]
    fn idle_after_new() {
        let bus = SpiBusState::new();
        assert_eq!(bus.bit_count, 0);
        assert_eq!(bus.bytes_exchanged, 0);
        assert!(bus.last_seln, "SELN starts high (deselected)");
        assert_eq!(bus.last_mosi_byte, None);
        assert!(bus.device.is_none());
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
        assert_eq!(bus.last_miso_byte, Some(0x00)); // no device attached
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

    #[test]
    fn echo_returns_seed_first_then_previous_mosi() {
        let mut bus = SpiBusState::new();
        attach_echo(&mut bus, 0xA5);
        bus.step(false, false, false, 0); // select → on_select returns 0xA5

        // Master clocks 0x11, slave drives 0xA5 (the seed).
        clock_byte(&mut bus, 0x11);
        assert_eq!(bus.last_mosi_byte, Some(0x11));
        assert_eq!(bus.last_miso_byte, Some(0xA5));

        // Master clocks 0x22, slave drives 0x11 (previously latched).
        clock_byte(&mut bus, 0x22);
        assert_eq!(bus.last_mosi_byte, Some(0x22));
        assert_eq!(bus.last_miso_byte, Some(0x11));

        // Master clocks 0x33, slave drives 0x22.
        clock_byte(&mut bus, 0x33);
        assert_eq!(bus.last_mosi_byte, Some(0x33));
        assert_eq!(bus.last_miso_byte, Some(0x22));

        assert_eq!(bus.bytes_exchanged, 3);
    }

    #[test]
    fn deselect_clears_miso_drive() {
        let mut bus = SpiBusState::new();
        attach_echo(&mut bus, 0xFF);
        bus.step(false, false, false, 0); // select
        clock_byte(&mut bus, 0x00);
        assert_eq!(bus.last_miso_byte, Some(0xFF));

        // Deselect: device's miso state in the bus should clear.
        bus.step(false, false, true, 0);
        assert_eq!(bus.shift_out, 0);
        assert_eq!(bus.bit_count, 0);

        // Reselect — on_select returns the latched buffer (0x00 from
        // the previous on_byte), so first MISO byte is 0x00.
        bus.step(false, false, false, 0);
        clock_byte(&mut bus, 0xAA);
        assert_eq!(bus.last_miso_byte, Some(0x00));
    }
}
