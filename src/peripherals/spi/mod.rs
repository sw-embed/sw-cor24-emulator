//! Layer-3 SPI: trait, typed-handle API, single-slave attachment slot,
//! and the concrete device implementations the bus dispatches to.

pub mod device;
pub mod devices;
pub mod handle;
pub mod registry;

pub use device::SpiDevice;
pub use devices::EchoDevice;
pub use handle::SpiHandle;
pub use registry::build_spi_device;
