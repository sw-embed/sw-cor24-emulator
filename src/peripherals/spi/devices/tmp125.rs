//! `tmp125` — TI TMP125 SPI temperature sensor.
//!
//! Models the chip's 16-bit shift-out register, where bits 14..5 hold
//! a 10-bit signed temperature in 0.25°C steps and bit 15 is fixed to
//! 0. Continuous reads cycle through (high byte, low byte, high byte,
//! low byte, ...) — the chip just keeps clocking out its register.
//!
//! Driven by the `tmp125.c` demo decode:
//! ```c
//! return (int)(((h << 3) | (l >> 5)) << 14) >> 14;
//! ```
//! That extracts the top 11 bits of (h<<8|l), then sign-extends from
//! bit 9 (= bit 14 of the 16-bit register), giving the 10-bit signed
//! value in 0.25°C steps. We synthesise h/l so the demo decode
//! recovers `set_temperature`'s value exactly.

use crate::peripherals::spi::device::SpiDevice;

pub struct Tmp125Device {
    /// Configured temperature in °C. Quantized at read time to 0.25°C
    /// steps via `temp_register`.
    temperature: f32,
    /// Whether the next read returns the high byte (true) or low byte
    /// (false). on_select resets this to true; toggles every byte.
    next_high: bool,
}

impl Default for Tmp125Device {
    fn default() -> Self {
        Self::new()
    }
}

impl Tmp125Device {
    pub fn new() -> Self {
        Self {
            temperature: 0.0,
            next_high: true,
        }
    }

    /// Set the configured temperature in °C. Quantized to 0.25°C
    /// steps at read time.
    pub fn set_temperature(&mut self, celsius: f32) {
        self.temperature = celsius;
    }

    /// Currently-configured temperature in °C as the demo would
    /// decode it (i.e., quantized to the chip's 0.25°C resolution).
    pub fn temperature(&self) -> f32 {
        let steps = self.steps();
        steps as f32 * 0.25
    }

    /// 10-bit signed temperature value in 0.25°C steps, clamped to the
    /// chip's representable range.
    fn steps(&self) -> i16 {
        let raw = (self.temperature / 0.25).round() as i32;
        raw.clamp(-512, 511) as i16
    }

    /// 16-bit register value the chip would clock out. Bit 15 is
    /// always 0; bits 14..5 hold the signed temperature in 0.25°C
    /// steps; bits 4..0 are zero.
    fn temp_register(&self) -> u16 {
        let val_10bit = (self.steps() & 0x03FF) as u16;
        val_10bit << 5
    }

    fn high_byte(&self) -> u8 {
        (self.temp_register() >> 8) as u8
    }

    fn low_byte(&self) -> u8 {
        self.temp_register() as u8
    }
}

impl SpiDevice for Tmp125Device {
    fn name(&self) -> &str {
        "tmp125"
    }

    fn on_select(&mut self) -> u8 {
        // First exchange after CS-low drives the high byte; the next
        // on_byte will drive the low byte. Reset the toggle so the
        // chip always starts a transaction with the high byte.
        self.next_high = false;
        self.high_byte()
    }

    fn on_byte(&mut self, _mosi: u8) -> u8 {
        let byte = if self.next_high {
            self.high_byte()
        } else {
            self.low_byte()
        };
        self.next_high = !self.next_high;
        byte
    }
}

/// Ergonomic extension on `SpiHandle<Tmp125Device>` so callers don't
/// need to spell out `handle.with(|d| d.set_temperature(c))`.
pub trait Tmp125HandleExt {
    fn set_temperature(&self, celsius: f32);
    fn temperature(&self) -> f32;
}

impl Tmp125HandleExt for crate::peripherals::spi::SpiHandle<Tmp125Device> {
    fn set_temperature(&self, celsius: f32) {
        self.with(|d| d.set_temperature(celsius));
    }
    fn temperature(&self) -> f32 {
        self.with(|d| d.temperature())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirror of the demo's `(((h << 3) | (l >> 5)) << 14) >> 14`
    /// decode, on a 24-bit signed `int`. Returns the recovered
    /// temperature in 0.25°C steps.
    fn demo_decode(h: u8, l: u8) -> i32 {
        let combined = ((h as u32) << 3) | ((l as u32) >> 5);
        // Sign-extend from bit 9 (= bit 14 of the 16-bit register).
        let v = (combined & 0x3FF) as i32;
        if v & 0x200 != 0 { v - 0x400 } else { v }
    }

    #[test]
    fn name_default() {
        let d = Tmp125Device::new();
        assert_eq!(d.name(), "tmp125");
    }

    #[test]
    fn zero_celsius_register() {
        let mut d = Tmp125Device::new();
        d.set_temperature(0.0);
        let h = d.on_select();
        let l = d.on_byte(0x00);
        assert_eq!((h, l), (0x00, 0x00));
        assert_eq!(demo_decode(h, l), 0);
    }

    #[test]
    fn twenty_five_celsius_register() {
        // 25.0°C = 100 steps. 10-bit signed = 0b 00 0110 0100 = 0x064.
        // Placed at bits 14..5: v = 0x064 << 5 = 0x0C80.
        // h = 0x0C, l = 0x80. demo_decode -> 100.
        let mut d = Tmp125Device::new();
        d.set_temperature(25.0);
        let h = d.on_select();
        let l = d.on_byte(0x00);
        assert_eq!((h, l), (0x0C, 0x80));
        assert_eq!(demo_decode(h, l), 100);
    }

    #[test]
    fn negative_ten_celsius_register() {
        // -10.0°C = -40 steps. 10-bit signed = 0b 11 1101 1000 = 0x3D8.
        // v = 0x3D8 << 5 = 0x7B00. h = 0x7B, l = 0x00.
        let mut d = Tmp125Device::new();
        d.set_temperature(-10.0);
        let h = d.on_select();
        let l = d.on_byte(0x00);
        assert_eq!((h, l), (0x7B, 0x00));
        assert_eq!(demo_decode(h, l), -40);
    }

    #[test]
    fn clamps_to_chip_range() {
        // Out-of-range temperatures clamp to the 10-bit signed range
        // (511 / -512 steps = 127.75°C / -128.00°C).
        let mut d = Tmp125Device::new();
        d.set_temperature(1000.0);
        assert_eq!(d.steps(), 511);
        d.set_temperature(-1000.0);
        assert_eq!(d.steps(), -512);
    }

    #[test]
    fn continuous_read_alternates_h_l() {
        // After on_select returns h, on_byte drives l, then h again,
        // then l, ... — matching the chip's continuous shift-out.
        let mut d = Tmp125Device::new();
        d.set_temperature(25.0);
        assert_eq!(d.on_select(), 0x0C);
        assert_eq!(d.on_byte(0x00), 0x80);
        assert_eq!(d.on_byte(0x00), 0x0C);
        assert_eq!(d.on_byte(0x00), 0x80);
    }

    #[test]
    fn reselect_resets_high_first() {
        // Even if a previous transaction left the toggle pointed at l,
        // a fresh on_select returns h and re-arms.
        let mut d = Tmp125Device::new();
        d.set_temperature(25.0);
        let _ = d.on_select();
        let _ = d.on_byte(0x00); // toggle now true (next_high)
        d.on_deselect();

        // Next transaction starts cleanly with high byte first.
        assert_eq!(d.on_select(), 0x0C);
        assert_eq!(d.on_byte(0x00), 0x80);
    }

    #[test]
    fn temperature_round_trip_quantized() {
        let mut d = Tmp125Device::new();
        d.set_temperature(25.0);
        assert_eq!(d.temperature(), 25.0);
        d.set_temperature(-10.5);
        assert_eq!(d.temperature(), -10.5);
        // Sub-step values quantize to the nearest 0.25°C step.
        d.set_temperature(25.10);
        assert_eq!(d.temperature(), 25.0);
        d.set_temperature(25.13);
        assert_eq!(d.temperature(), 25.25);
    }
}
