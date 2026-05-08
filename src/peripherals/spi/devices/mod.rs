//! Concrete SPI device implementations. Each device lives in its own
//! module; adding a new chip is one new file plus one line in
//! `registry::build_spi_device`.

pub mod echo;

pub use echo::EchoDevice;
