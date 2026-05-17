//! Concrete I2C device implementations. Each device lives in its own
//! module; adding a new chip is one new file plus one line in
//! `registry::build_i2c_device`.

pub mod add1;
pub mod ds1307;
pub mod ssd1306;
pub mod tmp101;

pub use add1::Add1Device;
pub use ds1307::{Ds1307Device, Ds1307HandleExt};
pub use ssd1306::{AddressingMode as Ssd1306AddressingMode, Ssd1306Device, Ssd1306HandleExt};
pub use tmp101::{Tmp101Device, Tmp101HandleExt, Tmp101Resolution};
