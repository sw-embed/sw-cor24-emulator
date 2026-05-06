//! Layer-3 I2C: trait, typed-handle API, address-routing table, and the
//! concrete device implementations the bus state machine routes to.

pub mod device;
pub mod devices;
pub mod handle;
pub mod registry;

pub use device::{Ack, I2cDevice};
pub use devices::{Add1Device, Tmp101Device, Tmp101HandleExt, Tmp101Resolution};
pub use handle::{AddressInUse, I2cHandle};
pub use registry::{AddressMap, build_i2c_device};
