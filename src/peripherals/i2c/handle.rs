//! Typed handle for runtime mutation of an attached I2C device.
//!
//! A handle wraps an `Arc<Mutex<D>>` plus a weak reference back to the
//! bus's address-routing table. Mutations through `with()` update the
//! device behind the bus's view (same `Arc`), and `set_address` updates
//! both the device's stored address and the routing table atomically.

use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, Weak};

use super::device::I2cDevice;
use super::registry::AddressMapInner;

/// Returned by `attach_i2c_device` / `I2cHandle::set_address` when an
/// address is already taken by another attached device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressInUse {
    pub address: u8,
}

impl fmt::Display for AddressInUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "I2C address 0x{:02X} is already in use", self.address)
    }
}

impl std::error::Error for AddressInUse {}

/// Typed handle to an attached I2C device.
///
/// `D` is the concrete device type; `with()` exposes it for chip-specific
/// mutation (e.g. `Tmp101Handle::set_temperature` is just sugar over
/// `handle.with(|d| d.set_temperature(...))`). The address-routing table
/// is reached through a `Weak` so the handle does not keep a stale bus
/// alive after the `EmulatorCore` is dropped.
pub struct I2cHandle<D: I2cDevice> {
    typed: Arc<Mutex<D>>,
    table: Weak<Mutex<AddressMapInner>>,
    _phantom: PhantomData<D>,
}

impl<D: I2cDevice> Clone for I2cHandle<D> {
    fn clone(&self) -> Self {
        Self {
            typed: self.typed.clone(),
            table: self.table.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<D: I2cDevice> fmt::Debug for I2cHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr = self
            .typed
            .lock()
            .map(|d| format!("0x{:02X}", d.address()))
            .unwrap_or_else(|_| "<poisoned>".into());
        f.debug_struct("I2cHandle")
            .field("address", &addr)
            .field("type", &std::any::type_name::<D>())
            .finish()
    }
}

impl<D: I2cDevice> I2cHandle<D> {
    pub(crate) fn new(typed: Arc<Mutex<D>>, table: Weak<Mutex<AddressMapInner>>) -> Self {
        Self {
            typed,
            table,
            _phantom: PhantomData,
        }
    }

    /// Run `f` with a mutable reference to the device. Bus's routing
    /// table is *not* refreshed — use `set_address` for that.
    pub fn with<R>(&self, f: impl FnOnce(&mut D) -> R) -> R {
        let mut guard = self.typed.lock().expect("I2cHandle: device lock poisoned");
        f(&mut *guard)
    }

    /// Current 7-bit I2C address.
    pub fn address(&self) -> u8 {
        self.typed
            .lock()
            .expect("I2cHandle: device lock poisoned")
            .address()
    }

    /// Move the device to a new 7-bit address, updating both the device
    /// and the bus's routing in one call.
    pub fn set_address(&self, addr: u8) -> Result<(), AddressInUse> {
        let table = self.table.upgrade().ok_or(AddressInUse { address: addr })?;
        let mut tab = table.lock().expect("I2C address table poisoned");
        let cur = self
            .typed
            .lock()
            .expect("I2cHandle: device lock poisoned")
            .address();
        if cur == addr {
            return Ok(());
        }
        if tab.entries.contains_key(&addr) {
            return Err(AddressInUse { address: addr });
        }
        let entry = tab
            .entries
            .remove(&cur)
            .expect("device missing from routing table");
        tab.entries.insert(addr, entry);
        self.typed
            .lock()
            .expect("I2cHandle: device lock poisoned")
            .set_address(addr);
        Ok(())
    }
}
