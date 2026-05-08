//! Typed handle for runtime mutation of the attached SPI device.
//!
//! Wraps an `Arc<Mutex<D>>` shared with the bus, so mutations through
//! `with()` are visible on the next exchange. SPI is single-slave today
//! (plan §9: future multi-slave bitmask), so there is no address
//! mutation — unlike `I2cHandle`.

use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use super::device::SpiDevice;

pub struct SpiHandle<D: SpiDevice> {
    typed: Arc<Mutex<D>>,
    _phantom: PhantomData<D>,
}

impl<D: SpiDevice> Clone for SpiHandle<D> {
    fn clone(&self) -> Self {
        Self {
            typed: self.typed.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<D: SpiDevice> fmt::Debug for SpiHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpiHandle")
            .field("type", &std::any::type_name::<D>())
            .finish()
    }
}

impl<D: SpiDevice> SpiHandle<D> {
    pub(crate) fn new(typed: Arc<Mutex<D>>) -> Self {
        Self {
            typed,
            _phantom: PhantomData,
        }
    }

    /// Run `f` with a mutable reference to the device.
    pub fn with<R>(&self, f: impl FnOnce(&mut D) -> R) -> R {
        let mut guard = self.typed.lock().expect("SpiHandle: device lock poisoned");
        f(&mut *guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::spi::devices::echo::EchoDevice;

    #[test]
    fn handle_with_round_trip() {
        let dev = EchoDevice::new(0x55);
        let arc: Arc<Mutex<EchoDevice>> = Arc::new(Mutex::new(dev));
        let handle = SpiHandle::new(arc);

        // Read seed via handle.
        assert_eq!(handle.with(|d| d.peek()), 0x55);

        // Mutate via handle, observe.
        handle.with(|d| d.poke(0xAB));
        assert_eq!(handle.with(|d| d.peek()), 0xAB);
    }
}
