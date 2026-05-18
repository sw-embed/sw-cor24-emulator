//! Concrete SPI device implementations. Each device lives in its own
//! module; adding a new chip is one new file plus one line in
//! `registry::build_spi_device`.

pub mod echo;
pub mod sdcard;
pub mod tmp125;

pub use echo::EchoDevice;
pub use sdcard::{SdCardDevice, SdCardHandleExt};
pub use tmp125::{Tmp125Device, Tmp125HandleExt};
