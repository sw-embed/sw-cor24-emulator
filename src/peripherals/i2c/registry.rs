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
        other => Err(format!("unknown I2C device '{other}'")),
    }
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
}
