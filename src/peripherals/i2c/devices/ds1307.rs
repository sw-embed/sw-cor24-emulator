//! `ds1307` — Maxim/Dallas DS1307 real-time clock (I2C).
//!
//! Eight BCD registers behind an auto-incrementing pointer, exactly as
//! the datasheet describes. The web UI's RTC panel reads/writes them
//! through the same bit-bang path a real master would use, so this
//! device mirrors the chip's protocol rather than offering a friendly
//! out-of-band interface.
//!
//! Pointer mapping (`pointer & 0x07`):
//!   - `0x00` — Seconds (00–59 BCD). Bit 7 = CH (Clock Halt) — masked
//!     out on writes so the emulated clock keeps ticking regardless.
//!   - `0x01` — Minutes (00–59 BCD).
//!   - `0x02` — Hours (00–23 BCD). Bit 6 = 12/24 mode; this device
//!     stores 24-hour format.
//!   - `0x03` — Day of Week (1–7 BCD).
//!   - `0x04` — Date (01–31 BCD).
//!   - `0x05` — Month (01–12 BCD).
//!   - `0x06` — Year (00–99 BCD).
//!   - `0x07` — Control. Stored verbatim; the SQW/RAM/OUT bits don't
//!     affect timekeeping in this emulation.
//!
//! ## Power-on default and "battery backup"
//!
//! A fresh `Ds1307Device::new(addr)` initializes every register to 0x00
//! — matching a real chip whose battery has never been installed: time
//! 00:00:00 on 00/00/00, CH bit clear, the master can read it but the
//! values are meaningless until the master writes the clock.
//!
//! Real DS1307 silicon keeps ticking on a coin-cell across main-power
//! cycles, but the chip itself has no notion of "wall clock" — its
//! crystal just counts oscillator ticks regardless of what year the
//! human at the bench thinks it is. We model this honestly:
//!
//! - The device exposes `set_from_unix_seconds(secs)` as the single
//!   public seed entry. It does *not* call `SystemTime::now()`
//!   internally — that would couple chip emulation to host wall-clock
//!   and break deterministic tests.
//! - `tick_second()` is the master-driven advance; the bus owner
//!   (web run-loop, CLI test, whatever) calls it once per emulated
//!   second.
//! - "Battery backup" is a *consumer-side* persistence concern. The
//!   web UI saves `(rtc_unix_secs, host_unix_secs)` to local storage
//!   on shutdown, and on next startup computes
//!   `current_rtc = saved_rtc + (now_host - saved_host)`, passing the
//!   result back through `set_from_unix_seconds`. The emulator stays
//!   pure; the persistence math lives where state actually persists.
//!
//! The CLI's `--i2c-device ds1307@<addr>?epoch=now` (or
//! `?epoch=<unix_seconds>`) is sugar for the same `set_from_unix_seconds`
//! path — the registry parser handles the `SystemTime::now()` call
//! when it sees `now`, so the host-clock dependency stays at the
//! consumer boundary.

use crate::peripherals::i2c::device::{Ack, I2cDevice};

/// Default 7-bit address for the DS1307.
pub const DEFAULT_ADDRESS: u8 = 0x68;

const REG_SECONDS: usize = 0;
const REG_MINUTES: usize = 1;
const REG_HOURS: usize = 2;
const REG_DAY_OF_WEEK: usize = 3;
const REG_DATE: usize = 4;
const REG_MONTH: usize = 5;
const REG_YEAR: usize = 6;
const REG_CONTROL: usize = 7;

const CH_BIT: u8 = 0x80;

pub struct Ds1307Device {
    address: u8,
    pointer: u8,
    /// Byte index within the current write transaction. 0 = next byte
    /// is the pointer load; 1+ = data bytes for `regs[pointer]`.
    write_idx: u8,
    /// Eight BCD registers: [sec, min, hr, dow, date, month, year, ctrl].
    regs: [u8; 8],
}

impl Ds1307Device {
    pub fn new(address: u8) -> Self {
        Self {
            address: address & 0x7F,
            pointer: 0,
            write_idx: 0,
            regs: [0; 8],
        }
    }

    /// Set the time-of-day in 24-hour binary form. Out-of-range values
    /// are clamped to the chip's permitted ranges before BCD-encoding.
    pub fn set_time(&mut self, hour: u8, minute: u8, second: u8) {
        // Clear CH on every set so the clock starts ticking from this
        // moment regardless of what the master had written earlier.
        self.regs[REG_SECONDS] = int_to_bcd(second.min(59));
        self.regs[REG_MINUTES] = int_to_bcd(minute.min(59));
        self.regs[REG_HOURS] = int_to_bcd(hour.min(23)); // bit 6 stays 0 = 24-hour
    }

    /// Set the calendar in binary form.
    pub fn set_date(&mut self, year: u8, month: u8, date: u8, day_of_week: u8) {
        self.regs[REG_YEAR] = int_to_bcd(year.min(99));
        self.regs[REG_MONTH] = int_to_bcd(month.clamp(1, 12));
        self.regs[REG_DATE] = int_to_bcd(date.clamp(1, 31));
        self.regs[REG_DAY_OF_WEEK] = int_to_bcd(day_of_week.clamp(1, 7));
    }

    /// Seed the chip from a Unix epoch timestamp (UTC seconds since
    /// 1970-01-01). Does full leap-year math during the day→YMD
    /// decomposition — required to map Unix seconds back to a calendar
    /// date correctly. (`tick_second()` itself stays leap-year-naive
    /// per the original brief; the two concerns are independent.)
    ///
    /// Years map as the DS1307's two-digit form: 2000→00, 2026→26, ...,
    /// wrapping mod 100 past 2099.
    ///
    /// This is the only public seed entry by design. Pulling the host
    /// wall-clock is a consumer concern: the CLI registry does it when
    /// it sees `?epoch=now`; the web UI does it (and the
    /// battery-backed-persistence math) when restoring from local
    /// storage. See the module docs for the persistence pattern.
    pub fn set_from_unix_seconds(&mut self, secs: u64) {
        let (year, month, date, dow, hour, minute, second) = decompose_unix_seconds(secs);
        self.set_date(year, month, date, dow);
        self.set_time(hour, minute, second);
    }

    pub fn second(&self) -> u8 {
        bcd_to_int(self.regs[REG_SECONDS] & !CH_BIT)
    }

    pub fn minute(&self) -> u8 {
        bcd_to_int(self.regs[REG_MINUTES])
    }

    pub fn hour(&self) -> u8 {
        // Mask off the 12/24 + AM/PM bits — this device runs in 24-hour
        // mode but a master could write bit 6 / bit 5; reporting plain
        // binary hours keeps the panel readout sane.
        bcd_to_int(self.regs[REG_HOURS] & 0x3F)
    }

    pub fn day_of_week(&self) -> u8 {
        bcd_to_int(self.regs[REG_DAY_OF_WEEK] & 0x07)
    }

    pub fn date(&self) -> u8 {
        bcd_to_int(self.regs[REG_DATE] & 0x3F)
    }

    pub fn month(&self) -> u8 {
        bcd_to_int(self.regs[REG_MONTH] & 0x1F)
    }

    pub fn year(&self) -> u8 {
        bcd_to_int(self.regs[REG_YEAR])
    }

    pub fn control(&self) -> u8 {
        self.regs[REG_CONTROL]
    }

    /// Snapshot of the eight raw register bytes, useful for tests and
    /// for the web panel's "show me everything" view.
    pub fn registers(&self) -> [u8; 8] {
        self.regs
    }

    /// Advance the clock by one second, cascading into minutes, hours,
    /// day-of-week, date, month, and year as needed. Leap-year handling
    /// is intentionally deferred — February is fixed at 28 days per the
    /// brief.
    pub fn tick_second(&mut self) {
        let mut sec = self.second();
        let mut min = self.minute();
        let mut hr = self.hour();
        let mut dow = self.day_of_week();
        let mut date = self.date();
        let mut month = self.month();
        let mut year = self.year();

        sec += 1;
        if sec >= 60 {
            sec = 0;
            min += 1;
            if min >= 60 {
                min = 0;
                hr += 1;
                if hr >= 24 {
                    hr = 0;
                    dow = if dow >= 7 { 1 } else { dow + 1 };
                    date += 1;
                    if date > days_in_month(month) {
                        date = 1;
                        month += 1;
                        if month > 12 {
                            month = 1;
                            year = (year + 1) % 100;
                        }
                    }
                }
            }
        }

        self.regs[REG_SECONDS] = int_to_bcd(sec);
        self.regs[REG_MINUTES] = int_to_bcd(min);
        self.regs[REG_HOURS] = int_to_bcd(hr);
        self.regs[REG_DAY_OF_WEEK] = int_to_bcd(dow);
        self.regs[REG_DATE] = int_to_bcd(date);
        self.regs[REG_MONTH] = int_to_bcd(month);
        self.regs[REG_YEAR] = int_to_bcd(year);
    }
}

fn int_to_bcd(n: u8) -> u8 {
    ((n / 10) << 4) | (n % 10)
}

fn bcd_to_int(b: u8) -> u8 {
    ((b >> 4) * 10) + (b & 0x0F)
}

fn days_in_month(month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28,
        _ => 31,
    }
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month_full(month: u32, year: i32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

/// Convert Unix epoch seconds (UTC) into the seven DS1307 fields:
/// `(year_2digit, month, date, day_of_week, hour, minute, second)`.
///
/// `year_2digit` is the year mod 100 starting from 2000 (so 2026 → 26).
/// `day_of_week` is 1..=7 with Sunday=1, matching one common DS1307
/// convention; 1970-01-01 was a Thursday, which lands on 5.
fn decompose_unix_seconds(secs: u64) -> (u8, u8, u8, u8, u8, u8, u8) {
    let days = secs / 86_400;
    let rem = (secs % 86_400) as u32;
    let hour = (rem / 3600) as u8;
    let minute = ((rem / 60) % 60) as u8;
    let second = (rem % 60) as u8;

    // Sunday=1. 1970-01-01 was Thursday → ((0 + 4) % 7) + 1 = 5.
    let dow = ((days + 4) % 7 + 1) as u8;

    let (year, month, date) = days_to_ymd(days);
    let year_2digit = (year - 2000).rem_euclid(100) as u8;
    (year_2digit, month as u8, date as u8, dow, hour, minute, second)
}

fn days_to_ymd(mut days: u64) -> (i32, u32, u32) {
    let mut year: i32 = 1970;
    loop {
        let yd = if is_leap_year(year) { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let mut month = 1u32;
    loop {
        let ml = days_in_month_full(month, year) as u64;
        if days < ml {
            break;
        }
        days -= ml;
        month += 1;
    }
    (year, month, days as u32 + 1)
}

impl I2cDevice for Ds1307Device {
    fn address(&self) -> u8 {
        self.address
    }

    fn set_address(&mut self, addr: u8) {
        self.address = addr & 0x7F;
    }

    fn name(&self) -> &str {
        "ds1307"
    }

    fn on_start(&mut self) {
        self.write_idx = 0;
    }

    fn on_write_byte(&mut self, byte: u8) -> Ack {
        if self.write_idx == 0 {
            self.pointer = byte & 0x07;
        } else {
            let value = if self.pointer as usize == REG_SECONDS {
                byte & !CH_BIT
            } else {
                byte
            };
            self.regs[self.pointer as usize] = value;
            self.pointer = (self.pointer + 1) & 0x07;
        }
        self.write_idx = self.write_idx.saturating_add(1);
        Ack::Ack
    }

    fn on_read_byte(&mut self) -> u8 {
        let byte = self.regs[self.pointer as usize];
        self.pointer = (self.pointer + 1) & 0x07;
        byte
    }
}

/// Ergonomic extension on `I2cHandle<Ds1307Device>` mirroring the
/// `Tmp101HandleExt` shape.
pub trait Ds1307HandleExt {
    fn set_time(&self, hour: u8, minute: u8, second: u8);
    fn set_date(&self, year: u8, month: u8, date: u8, day_of_week: u8);
    fn set_from_unix_seconds(&self, secs: u64);
    fn hour(&self) -> u8;
    fn minute(&self) -> u8;
    fn second(&self) -> u8;
    fn day_of_week(&self) -> u8;
    fn date(&self) -> u8;
    fn month(&self) -> u8;
    fn year(&self) -> u8;
    fn tick_second(&self);
}

impl Ds1307HandleExt for crate::peripherals::i2c::I2cHandle<Ds1307Device> {
    fn set_time(&self, hour: u8, minute: u8, second: u8) {
        self.with(|d| d.set_time(hour, minute, second));
    }
    fn set_date(&self, year: u8, month: u8, date: u8, day_of_week: u8) {
        self.with(|d| d.set_date(year, month, date, day_of_week));
    }
    fn set_from_unix_seconds(&self, secs: u64) {
        self.with(|d| d.set_from_unix_seconds(secs));
    }
    fn hour(&self) -> u8 {
        self.with(|d| d.hour())
    }
    fn minute(&self) -> u8 {
        self.with(|d| d.minute())
    }
    fn second(&self) -> u8 {
        self.with(|d| d.second())
    }
    fn day_of_week(&self) -> u8 {
        self.with(|d| d.day_of_week())
    }
    fn date(&self) -> u8 {
        self.with(|d| d.date())
    }
    fn month(&self) -> u8 {
        self.with(|d| d.month())
    }
    fn year(&self) -> u8 {
        self.with(|d| d.year())
    }
    fn tick_second(&self) {
        self.with(|d| d.tick_second());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_default_address() {
        let d = Ds1307Device::new(DEFAULT_ADDRESS);
        assert_eq!(d.name(), "ds1307");
        assert_eq!(d.address(), 0x68);
    }

    #[test]
    fn bcd_round_trip() {
        for n in 0u8..=99 {
            assert_eq!(bcd_to_int(int_to_bcd(n)), n, "round-trip for {n}");
        }
        // Spot-check the documented encoding (45 → 0x45).
        assert_eq!(int_to_bcd(45), 0x45);
        assert_eq!(bcd_to_int(0x45), 45);
    }

    #[test]
    fn set_time_and_set_date_store_bcd() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(14, 45, 30);
        d.set_date(26, 5, 16, 6); // 2026-05-16, Saturday
        let regs = d.registers();
        assert_eq!(regs[REG_SECONDS], 0x30);
        assert_eq!(regs[REG_MINUTES], 0x45);
        assert_eq!(regs[REG_HOURS], 0x14);
        assert_eq!(regs[REG_DAY_OF_WEEK], 0x06);
        assert_eq!(regs[REG_DATE], 0x16);
        assert_eq!(regs[REG_MONTH], 0x05);
        assert_eq!(regs[REG_YEAR], 0x26);

        assert_eq!(d.hour(), 14);
        assert_eq!(d.minute(), 45);
        assert_eq!(d.second(), 30);
        assert_eq!(d.year(), 26);
    }

    #[test]
    fn sequential_read_from_pointer_zero_returns_seven_bytes_in_order() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(14, 45, 30);
        d.set_date(26, 5, 16, 6);

        // Master writes pointer = 0x00, then issues repeated START for read.
        d.on_start();
        assert_eq!(d.on_write_byte(0x00), Ack::Ack);
        d.on_start();
        let bytes: Vec<u8> = (0..7).map(|_| d.on_read_byte()).collect();
        assert_eq!(bytes, vec![0x30, 0x45, 0x14, 0x06, 0x16, 0x05, 0x26]);
    }

    #[test]
    fn pointer_wraps_after_register_seven() {
        let mut d = Ds1307Device::new(0x68);
        d.regs = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

        d.on_start();
        assert_eq!(d.on_write_byte(0x07), Ack::Ack); // pointer = control
        d.on_start();
        assert_eq!(d.on_read_byte(), 0x77); // control
        assert_eq!(d.on_read_byte(), 0x00); // wraps to seconds
        assert_eq!(d.on_read_byte(), 0x11); // minutes
    }

    #[test]
    fn pointer_load_masks_to_low_three_bits() {
        let mut d = Ds1307Device::new(0x68);
        d.regs = [0xAA, 0, 0, 0, 0, 0, 0, 0xBB];

        d.on_start();
        // 0xF8 → pointer = 0x00 after masking
        assert_eq!(d.on_write_byte(0xF8), Ack::Ack);
        d.on_start();
        assert_eq!(d.on_read_byte(), 0xAA);
    }

    #[test]
    fn pointer_load_then_seven_data_bytes_lands_in_regs() {
        let mut d = Ds1307Device::new(0x68);
        d.on_start();
        assert_eq!(d.on_write_byte(0x00), Ack::Ack); // pointer
        for b in [0x00u8, 0x15, 0x09, 0x03, 0x16, 0x05, 0x26] {
            assert_eq!(d.on_write_byte(b), Ack::Ack);
        }
        // Datasheet example from the research doc.
        assert_eq!(d.second(), 0);
        assert_eq!(d.minute(), 15);
        assert_eq!(d.hour(), 9);
        assert_eq!(d.day_of_week(), 3);
        assert_eq!(d.date(), 16);
        assert_eq!(d.month(), 5);
        assert_eq!(d.year(), 26);
    }

    #[test]
    fn ch_bit_masked_on_writes_to_seconds() {
        let mut d = Ds1307Device::new(0x68);
        d.on_start();
        assert_eq!(d.on_write_byte(0x00), Ack::Ack); // pointer = seconds
        assert_eq!(d.on_write_byte(0xC5), Ack::Ack); // CH=1, "45" seconds
        assert_eq!(d.registers()[REG_SECONDS], 0x45);
        assert_eq!(d.second(), 45);
    }

    #[test]
    fn tick_second_cascades_minute() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(1, 0, 59);
        d.tick_second();
        assert_eq!(d.second(), 0);
        assert_eq!(d.minute(), 1);
        assert_eq!(d.hour(), 1);
    }

    #[test]
    fn tick_second_cascades_hour() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(1, 59, 59);
        d.tick_second();
        assert_eq!(d.second(), 0);
        assert_eq!(d.minute(), 0);
        assert_eq!(d.hour(), 2);
    }

    #[test]
    fn tick_second_cascades_day_at_midnight() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 5, 16, 6); // Sat May 16
        d.tick_second();
        assert_eq!(d.hour(), 0);
        assert_eq!(d.minute(), 0);
        assert_eq!(d.second(), 0);
        assert_eq!(d.date(), 17);
        assert_eq!(d.day_of_week(), 7);
    }

    #[test]
    fn tick_second_day_of_week_wraps_after_seven() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 5, 16, 7); // Saturday in DS1307's 1..=7
        d.tick_second();
        assert_eq!(d.day_of_week(), 1);
    }

    #[test]
    fn tick_second_end_of_31_day_month_rolls_into_next() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 5, 31, 6); // May 31 → June 1
        d.tick_second();
        assert_eq!(d.date(), 1);
        assert_eq!(d.month(), 6);
    }

    #[test]
    fn tick_second_end_of_30_day_month_rolls_into_next() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 4, 30, 1); // Apr 30 → May 1
        d.tick_second();
        assert_eq!(d.date(), 1);
        assert_eq!(d.month(), 5);
    }

    #[test]
    fn tick_second_end_of_february_28_days() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 2, 28, 7); // Feb 28 → Mar 1 (leap years deferred)
        d.tick_second();
        assert_eq!(d.date(), 1);
        assert_eq!(d.month(), 3);
    }

    #[test]
    fn tick_second_end_of_year_rolls_over() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(26, 12, 31, 5);
        d.tick_second();
        assert_eq!(d.date(), 1);
        assert_eq!(d.month(), 1);
        assert_eq!(d.year(), 27);
    }

    #[test]
    fn tick_second_year_99_wraps_to_zero() {
        let mut d = Ds1307Device::new(0x68);
        d.set_time(23, 59, 59);
        d.set_date(99, 12, 31, 1);
        d.tick_second();
        assert_eq!(d.year(), 0);
    }

    #[test]
    fn write_after_read_continues_at_advanced_pointer() {
        // DS1307 advances the pointer on read; a subsequent write should
        // start writing at the new pointer, not at zero.
        let mut d = Ds1307Device::new(0x68);
        d.regs = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];

        d.on_start();
        assert_eq!(d.on_write_byte(0x04), Ack::Ack); // pointer = date
        d.on_start();
        assert_eq!(d.on_read_byte(), 0x50); // date, pointer advances to month
        d.on_start();
        // Now a write transaction: first byte is pointer load again.
        assert_eq!(d.on_write_byte(0x05), Ack::Ack);
        assert_eq!(d.on_write_byte(0x07), Ack::Ack); // month = 0x07
        assert_eq!(d.registers()[REG_MONTH], 0x07);
    }

    #[test]
    fn set_address_updates_responding_address() {
        let mut d = Ds1307Device::new(0x68);
        d.set_address(0x42);
        assert_eq!(d.address(), 0x42);
        d.set_address(0xFF); // high bit cleared
        assert_eq!(d.address(), 0x7F);
    }

    #[test]
    fn decompose_unix_epoch() {
        // 1970-01-01 00:00:00 UTC, Thursday (Sunday=1 → Thu=5).
        let (y, m, da, dow, h, mi, s) = decompose_unix_seconds(0);
        // Year mod 100 from 2000: 1970 → (1970-2000) mod 100 = 70.
        assert_eq!((y, m, da, dow, h, mi, s), (70, 1, 1, 5, 0, 0, 0));
    }

    #[test]
    fn decompose_2026_05_16_noon() {
        // 2026-05-16 12:00:00 UTC = 1_778_932_800 (Saturday).
        // Confirmed via `date -u -d "2026-05-16 12:00:00 UTC" +%s`.
        let (y, m, da, dow, h, mi, s) = decompose_unix_seconds(1_778_932_800);
        assert_eq!((y, m, da, h, mi, s), (26, 5, 16, 12, 0, 0));
        // Sunday=1 .. Saturday=7.
        assert_eq!(dow, 7);
    }

    #[test]
    fn decompose_handles_leap_day_2024_02_29() {
        // 2024-02-29 00:00:00 UTC = 1_709_164_800.
        let (y, m, da, _, h, mi, s) = decompose_unix_seconds(1_709_164_800);
        assert_eq!((y, m, da, h, mi, s), (24, 2, 29, 0, 0, 0));
    }

    #[test]
    fn decompose_handles_post_leap_2024_03_01() {
        // 2024-03-01 00:00:00 UTC = 1_709_251_200.
        let (y, m, da, _, h, mi, s) = decompose_unix_seconds(1_709_251_200);
        assert_eq!((y, m, da, h, mi, s), (24, 3, 1, 0, 0, 0));
    }

    #[test]
    fn decompose_year_2100_not_leap_feb_28_to_mar_1() {
        // 2100-02-28 23:59:59 UTC = 4_107_542_399. Year 2100 is NOT a
        // leap year (divisible by 100 but not 400), so Feb has 28 days.
        let (y, m, da, _, h, mi, s) = decompose_unix_seconds(4_107_542_399);
        // Year wraps mod 100 → 2100 maps to 0.
        assert_eq!((y, m, da, h, mi, s), (0, 2, 28, 23, 59, 59));
        let (y, m, da, _, _, _, _) = decompose_unix_seconds(4_107_542_399 + 1);
        assert_eq!((y, m, da), (0, 3, 1));
    }

    #[test]
    fn set_from_unix_seconds_round_trip() {
        let mut d = Ds1307Device::new(0x68);
        // 2026-05-16 14:45:30 UTC = 1_778_942_730.
        d.set_from_unix_seconds(1_778_942_730);
        assert_eq!(d.year(), 26);
        assert_eq!(d.month(), 5);
        assert_eq!(d.date(), 16);
        assert_eq!(d.hour(), 14);
        assert_eq!(d.minute(), 45);
        assert_eq!(d.second(), 30);
        assert_eq!(d.day_of_week(), 7); // Saturday
    }

    #[test]
    fn default_chip_state_is_zero_no_implicit_host_clock() {
        // A fresh device matches "no-battery / cold-start hardware" —
        // every register zero, no leak from the host's wall clock.
        let d = Ds1307Device::new(0x68);
        assert_eq!(d.registers(), [0; 8]);
        assert_eq!(d.year(), 0);
        assert_eq!(d.hour(), 0);
        assert_eq!(d.second(), 0);
    }
}
