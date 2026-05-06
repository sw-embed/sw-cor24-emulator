//! Concrete I2C device implementations. Each device lives in its own
//! module; adding a new chip is one new file plus one line in
//! `registry::build_i2c_device`.

pub mod add1;

pub use add1::Add1Device;
