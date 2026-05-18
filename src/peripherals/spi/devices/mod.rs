//! Concrete SPI device implementations. Each device lives in its own
//! module; adding a new chip is one new file plus one line in
//! `registry::build_spi_device`.

pub mod echo;
pub mod sdcard;
pub mod tmp125;
pub mod w25q32;

pub use echo::EchoDevice;
pub use sdcard::{SdCardDevice, SdCardHandleExt};
pub use tmp125::{Tmp125Device, Tmp125HandleExt};
pub use w25q32::{W25q32Device, W25q32HandleExt};
