//! Pluggable peripherals — virtual devices that attach to the
//! emulator's MMIO buses (I2C now, SPI later). Sits at the same level
//! as `assembler` and `loader` per plan §4.3 because peripherals are
//! not CPU concerns.

pub mod i2c;
