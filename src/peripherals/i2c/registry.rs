//! I2C address-routing table and CLI device registry stub.
//!
//! `AddressMap` is what the bus state machine consults on every byte
//! completion to decide whether to ACK and which device receives the
//! event. It is wrapped in `Arc<Mutex<...>>` so the typed `I2cHandle`
//! can refresh routing on `set_address`.
//!
//! `build_i2c_device` is the string-keyed registry the CLI will parse
//! (`add1@0x50`, `tmp101@0x4A`, ...). For now the only known device is
//! `add1`; additional devices land in their own steps.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::device::I2cDevice;
use super::devices::add1::Add1Device;
use super::devices::tmp101::Tmp101Device;

/// Inner storage of the routing table. Public to the crate so the
/// typed handle can mutate it on `set_address`.
#[derive(Default)]
pub struct AddressMapInner {
    pub entries: HashMap<u8, Arc<Mutex<dyn I2cDevice>>>,
}

/// Shared address-routing table. Cloning shares the same allocation so
/// the bus state and any number of `I2cHandle`s see the same routing.
#[derive(Clone, Default)]
pub struct AddressMap {
    inner: Arc<Mutex<AddressMapInner>>,
}

impl AddressMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn shared(&self) -> Arc<Mutex<AddressMapInner>> {
        self.inner.clone()
    }

    /// Look up the device at `addr`, returning a fresh `Arc` clone so
    /// the caller can drop the table lock before locking the device.
    pub fn lookup(&self, addr: u8) -> Option<Arc<Mutex<dyn I2cDevice>>> {
        self.inner.lock().ok()?.entries.get(&addr).cloned()
    }

    /// Insert the device at `addr`. Returns `Err(AddressInUse)` if the
    /// slot is already taken.
    pub(crate) fn insert(
        &self,
        addr: u8,
        dev: Arc<Mutex<dyn I2cDevice>>,
    ) -> Result<(), super::handle::AddressInUse> {
        let mut g = self.inner.lock().expect("I2C address table poisoned");
        if g.entries.contains_key(&addr) {
            return Err(super::handle::AddressInUse { address: addr });
        }
        g.entries.insert(addr, dev);
        Ok(())
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.entries.clear();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .lock()
            .map(|g| g.entries.is_empty())
            .unwrap_or(true)
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.entries.len()).unwrap_or(0)
    }
}

/// Parse a CLI device spec and construct the device, returning the
/// shared `Arc<Mutex<dyn I2cDevice>>` form the bus's address-routing
/// table holds. Callers wanting a typed handle should use
/// `EmulatorCore::attach_i2c_device(D::new(...))` directly instead.
///
/// Spec syntax: `<name>@<addr>[?key=val&...]`. Address is 7-bit hex
/// (`0x50` or `50`). Recognised devices:
///   - `add1@<addr>[?wrap=<n>]`             — universal +1 test slave.
///   - `tmp101@<addr>[?temp=<f>][?config=<n>]` — TI temp sensor.
///   - `ds1307@<addr>[?hour=<n>][?minute=<n>][?second=<n>]`
///     `[?date=<n>][?month=<n>][?year=<n>][?dow=<n>][?preset=system]`
///     — Dallas/Maxim RTC. Per-field params seed individual registers;
///     `preset=system` reads the host wall-clock at attach time and
///     fills all 7 time registers. `preset` and any per-field param
///     are mutually exclusive. Default (no params) is all-zero
///     registers (cold-start hardware behaviour).
pub fn build_i2c_device(
    spec: &str,
) -> Result<std::sync::Arc<std::sync::Mutex<dyn I2cDevice>>, String> {
    use std::sync::{Arc, Mutex};
    let (name_addr, params) = match spec.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (spec, None),
    };
    let (name, addr_str) = name_addr
        .split_once('@')
        .ok_or_else(|| format!("device spec missing '@<addr>': {spec}"))?;
    let addr = parse_addr(addr_str)
        .ok_or_else(|| format!("invalid 7-bit address in spec '{spec}'"))?;
    match name {
        "add1" => {
            let mut wrap: u16 = 0x100;
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "wrap" => {
                            wrap = v.parse().map_err(|e| format!("bad wrap '{v}': {e}"))?
                        }
                        _ => return Err(format!("unknown add1 param '{k}' in '{spec}'")),
                    }
                }
            }
            Ok(Arc::new(Mutex::new(Add1Device::new(addr, wrap))))
        }
        "tmp101" => {
            let mut dev = Tmp101Device::new(addr);
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "temp" => {
                            let c: f32 =
                                v.parse().map_err(|e| format!("bad temp '{v}': {e}"))?;
                            dev.set_temperature(c);
                        }
                        "config" => {
                            let c: u8 = if let Some(rest) =
                                v.strip_prefix("0x").or_else(|| v.strip_prefix("0X"))
                            {
                                u8::from_str_radix(rest, 16)
                                    .map_err(|e| format!("bad config '{v}': {e}"))?
                            } else {
                                v.parse().map_err(|e| format!("bad config '{v}': {e}"))?
                            };
                            dev.set_config(c);
                        }
                        _ => return Err(format!("unknown tmp101 param '{k}' in '{spec}'")),
                    }
                }
            }
            Ok(Arc::new(Mutex::new(dev)))
        }
        "ds1307" => build_ds1307(addr, params, spec),
        other => Err(format!("unknown I2C device '{other}'")),
    }
}

fn build_ds1307(
    addr: u8,
    params: Option<&str>,
    spec: &str,
) -> Result<Arc<Mutex<dyn I2cDevice>>, String> {
    use crate::peripherals::i2c::devices::ds1307::{Ds1307Device, int_to_bcd};

    let mut second: Option<u8> = None;
    let mut minute: Option<u8> = None;
    let mut hour: Option<u8> = None;
    let mut day_of_week: Option<u8> = None;
    let mut date: Option<u8> = None;
    let mut month: Option<u8> = None;
    let mut year: Option<u8> = None;
    let mut preset_seen = false;

    if let Some(p) = params {
        for kv in p.split('&') {
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
            match k {
                "hour" => hour = Some(parse_ds1307_range(k, v, 0, 23, spec)?),
                "minute" => minute = Some(parse_ds1307_range(k, v, 0, 59, spec)?),
                "second" => second = Some(parse_ds1307_range(k, v, 0, 59, spec)?),
                "date" => date = Some(parse_ds1307_range(k, v, 1, 31, spec)?),
                "month" => month = Some(parse_ds1307_range(k, v, 1, 12, spec)?),
                "year" => year = Some(parse_ds1307_range(k, v, 0, 99, spec)?),
                "dow" => day_of_week = Some(parse_ds1307_range(k, v, 1, 7, spec)?),
                "preset" => {
                    if v != "system" {
                        return Err(format!(
                            "ds1307 'preset' value '{v}' unknown (valid: 'system') in '{spec}'"
                        ));
                    }
                    preset_seen = true;
                }
                _ => return Err(format!("unknown ds1307 param '{k}' in '{spec}'")),
            }
        }
    }

    let has_field = hour.is_some()
        || minute.is_some()
        || second.is_some()
        || date.is_some()
        || month.is_some()
        || year.is_some()
        || day_of_week.is_some();

    if preset_seen && has_field {
        return Err(format!(
            "ds1307 'preset' and explicit register values are mutually exclusive in '{spec}'"
        ));
    }

    if preset_seen {
        let mut dev = Ds1307Device::new(addr);
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        dev.set_from_unix_seconds(secs);
        return Ok(Arc::new(Mutex::new(dev)));
    }

    let regs = [
        second.map(int_to_bcd).unwrap_or(0),
        minute.map(int_to_bcd).unwrap_or(0),
        hour.map(int_to_bcd).unwrap_or(0),
        day_of_week.map(int_to_bcd).unwrap_or(0),
        date.map(int_to_bcd).unwrap_or(0),
        month.map(int_to_bcd).unwrap_or(0),
        year.map(int_to_bcd).unwrap_or(0),
        0, // control: brief defers; stays zero
    ];
    Ok(Arc::new(Mutex::new(Ds1307Device::with_initial_registers(
        addr, regs,
    ))))
}

fn parse_ds1307_range(key: &str, value: &str, lo: u8, hi: u8, spec: &str) -> Result<u8, String> {
    let parsed: u32 = value
        .parse()
        .map_err(|_| format!("ds1307 '{key}' not a decimal number: '{value}' in '{spec}'"))?;
    let n = u8::try_from(parsed).map_err(|_| {
        format!("ds1307 '{key}' out of range: {parsed} (valid: {lo}-{hi}) in '{spec}'")
    })?;
    if !(lo..=hi).contains(&n) {
        return Err(format!(
            "ds1307 '{key}' out of range: {n} (valid: {lo}-{hi}) in '{spec}'"
        ));
    }
    Ok(n)
}

fn parse_addr(s: &str) -> Option<u8> {
    let s = s.trim();
    let n: u32 = if let Some(stripped) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(stripped, 16).ok()?
    } else {
        s.parse().ok()?
    };
    if n > 0x7F { None } else { Some(n as u8) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_address(spec: &str) -> u8 {
        let arc = build_i2c_device(spec).unwrap();
        let g = arc.lock().unwrap();
        g.address()
    }

    fn dev_name(spec: &str) -> String {
        let arc = build_i2c_device(spec).unwrap();
        let g = arc.lock().unwrap();
        g.name().to_string()
    }

    #[test]
    fn build_add1_default_wrap() {
        assert_eq!(dev_address("add1@0x50"), 0x50);
        assert_eq!(dev_name("add1@0x50"), "add1");
    }

    #[test]
    fn build_add1_with_wrap() {
        assert_eq!(dev_address("add1@0x42?wrap=10"), 0x42);
    }

    fn expect_err(spec: &str, needle: &str) {
        match build_i2c_device(spec) {
            Ok(_) => panic!("expected '{spec}' to fail"),
            Err(e) => assert!(e.contains(needle), "spec '{spec}' err: {e}"),
        }
    }

    #[test]
    fn build_unknown_device_rejected() {
        expect_err("frobnicator@0x50", "frobnicator");
    }

    #[test]
    fn build_invalid_address_rejected() {
        expect_err("add1@0xFF", "invalid");
    }

    #[test]
    fn build_missing_at_rejected() {
        expect_err("add1", "missing '@");
    }

    #[test]
    fn build_tmp101_default() {
        assert_eq!(dev_address("tmp101@0x4A"), 0x4A);
        assert_eq!(dev_name("tmp101@0x4A"), "tmp101");
    }

    #[test]
    fn build_tmp101_with_temperature() {
        let _ = build_i2c_device("tmp101@0x4A?temp=23.5").unwrap();
    }

    #[test]
    fn build_tmp101_with_config() {
        let _ = build_i2c_device("tmp101@0x4A?config=0x60").unwrap();
        let _ = build_i2c_device("tmp101@0x4A?config=32").unwrap();
    }

    #[test]
    fn build_tmp101_unknown_param_rejected() {
        expect_err("tmp101@0x4A?wrap=10", "unknown tmp101 param");
    }

    #[test]
    fn build_ds1307_default_address_and_name() {
        assert_eq!(dev_address("ds1307@0x68"), 0x68);
        assert_eq!(dev_name("ds1307@0x68"), "ds1307");
    }

    #[test]
    fn build_ds1307_default_is_zero() {
        // Default (no params) must mirror cold-start hardware: every
        // register reads as 0x00.
        let arc = build_i2c_device("ds1307@0x68").unwrap();
        let mut g = arc.lock().unwrap();
        g.on_start();
        assert_eq!(g.on_write_byte(0x00), crate::peripherals::i2c::Ack::Ack);
        g.on_start();
        for _ in 0..8 {
            assert_eq!(g.on_read_byte(), 0x00);
        }
    }

    /// Helper: read the first `n` registers of a built ds1307 device
    /// through the bus, returning raw BCD bytes.
    fn read_ds1307_bytes(spec: &str, n: usize) -> Vec<u8> {
        let arc = build_i2c_device(spec).unwrap();
        let mut g = arc.lock().unwrap();
        g.on_start();
        assert_eq!(g.on_write_byte(0x00), crate::peripherals::i2c::Ack::Ack);
        g.on_start();
        (0..n).map(|_| g.on_read_byte()).collect()
    }

    #[test]
    fn build_ds1307_with_time_params() {
        // hour=12, minute=34, second=56 → registers [0x56, 0x34, 0x12, ...].
        let bytes = read_ds1307_bytes("ds1307@0x68?hour=12&minute=34&second=56", 3);
        assert_eq!(bytes, vec![0x56, 0x34, 0x12]);
    }

    #[test]
    fn build_ds1307_with_full_date() {
        // All 7 time keys; verify each register encodes correctly.
        let spec = "ds1307@0x68?second=30&minute=45&hour=14&dow=7&date=16&month=5&year=26";
        let bytes = read_ds1307_bytes(spec, 7);
        assert_eq!(bytes, vec![0x30, 0x45, 0x14, 0x07, 0x16, 0x05, 0x26]);
    }

    #[test]
    fn build_ds1307_preset_system() {
        // preset=system reads SystemTime::now() at construction. We can't
        // pin the host clock, but year ≥ 0x25 (2025, our floor) confirms
        // the path executed. Generous tolerance for slow CI runners.
        let bytes = read_ds1307_bytes("ds1307@0x68?preset=system", 7);
        // bytes layout: [sec, min, hr, dow, date, mon, yr].
        let year_bcd = bytes[6];
        assert!(
            year_bcd >= 0x25,
            "expected preset=system to seed at least year 2025, got BCD {year_bcd:#04x}"
        );
        // Spot-check: month is 1-12 BCD; date is 1-31 BCD; hour < 24.
        assert!((0x01..=0x12).contains(&bytes[5]), "month BCD: {:#04x}", bytes[5]);
        assert!(bytes[2] <= 0x23, "hour BCD: {:#04x}", bytes[2]);
    }

    #[test]
    fn build_ds1307_preset_conflicts_with_hour() {
        expect_err(
            "ds1307@0x68?preset=system&hour=12",
            "ds1307 'preset' and explicit register values are mutually exclusive",
        );
    }

    #[test]
    fn build_ds1307_preset_conflicts_with_dow() {
        // Reverse order also detected.
        expect_err(
            "ds1307@0x68?dow=3&preset=system",
            "ds1307 'preset' and explicit register values are mutually exclusive",
        );
    }

    #[test]
    fn build_ds1307_out_of_range_rejected() {
        expect_err("ds1307@0x68?hour=24", "'hour' out of range: 24");
        expect_err("ds1307@0x68?minute=60", "'minute' out of range: 60");
        expect_err("ds1307@0x68?second=60", "'second' out of range: 60");
        expect_err("ds1307@0x68?date=0", "'date' out of range: 0");
        expect_err("ds1307@0x68?date=32", "'date' out of range: 32");
        expect_err("ds1307@0x68?month=0", "'month' out of range: 0");
        expect_err("ds1307@0x68?month=13", "'month' out of range: 13");
        expect_err("ds1307@0x68?year=100", "'year' out of range: 100");
        expect_err("ds1307@0x68?dow=0", "'dow' out of range: 0");
        expect_err("ds1307@0x68?dow=8", "'dow' out of range: 8");
    }

    #[test]
    fn build_ds1307_unknown_preset_value_rejected() {
        expect_err(
            "ds1307@0x68?preset=zero",
            "ds1307 'preset' value 'zero' unknown",
        );
    }

    #[test]
    fn build_ds1307_non_numeric_field_rejected() {
        expect_err(
            "ds1307@0x68?hour=twelve",
            "ds1307 'hour' not a decimal number",
        );
    }

    #[test]
    fn build_ds1307_unknown_param_rejected() {
        expect_err("ds1307@0x68?temp=25.0", "unknown ds1307 param");
    }

    #[test]
    fn build_ds1307_partial_fields_default_unspecified_to_zero() {
        // hour=12 alone leaves minute, second, date, month, year, dow at
        // their default zero — matching the brief's "all-zero default"
        // and what cold-start hardware would give.
        let bytes = read_ds1307_bytes("ds1307@0x68?hour=12", 7);
        assert_eq!(bytes, vec![0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00]);
    }
}
