//! CLI device registry for SPI.
//!
//! `build_spi_device` is the string-keyed registry the CLI parses. SPI
//! is single-slave today (plan §9: future multi-slave SELN bitmask),
//! so spec syntax has no `@<addr>` — `<name>[?key=val&...]`.
//!
//! Recognised devices:
//!   - `echo[?seed=<n>]`            — universal echo test slave.
//!   - `tmp125[?temp=<f>]`          — TI TMP125 temperature sensor.

use std::sync::{Arc, Mutex};

use super::device::SpiDevice;
use super::devices::echo::EchoDevice;
use super::devices::tmp125::Tmp125Device;

pub fn build_spi_device(spec: &str) -> Result<Arc<Mutex<dyn SpiDevice>>, String> {
    let (name, params) = match spec.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (spec, None),
    };
    match name {
        "echo" => {
            let mut seed: u8 = 0;
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "seed" => {
                            seed = parse_u8(v)
                                .ok_or_else(|| format!("bad seed '{v}' in '{spec}'"))?;
                        }
                        _ => return Err(format!("unknown echo param '{k}' in '{spec}'")),
                    }
                }
            }
            Ok(Arc::new(Mutex::new(EchoDevice::new(seed))))
        }
        "tmp125" => {
            let mut dev = Tmp125Device::new();
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "temp" => {
                            let c: f32 =
                                v.parse().map_err(|e| format!("bad temp '{v}': {e}"))?;
                            dev.set_temperature(c);
                        }
                        _ => return Err(format!("unknown tmp125 param '{k}' in '{spec}'")),
                    }
                }
            }
            Ok(Arc::new(Mutex::new(dev)))
        }
        other => Err(format!("unknown SPI device '{other}'")),
    }
}

fn parse_u8(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u8::from_str_radix(rest, 16).ok()
    } else {
        s.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_echo_default_seed() {
        let arc = build_spi_device("echo").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "echo");
    }

    #[test]
    fn build_echo_with_decimal_seed() {
        let arc = build_spi_device("echo?seed=42").unwrap();
        let mut g = arc.lock().unwrap();
        assert_eq!(g.on_select(), 42);
    }

    #[test]
    fn build_echo_with_hex_seed() {
        let arc = build_spi_device("echo?seed=0xAB").unwrap();
        let mut g = arc.lock().unwrap();
        assert_eq!(g.on_select(), 0xAB);
    }

    fn expect_err(spec: &str) -> String {
        match build_spi_device(spec) {
            Ok(_) => panic!("expected '{spec}' to fail"),
            Err(e) => e,
        }
    }

    #[test]
    fn build_tmp125_default() {
        let arc = build_spi_device("tmp125").unwrap();
        let mut g = arc.lock().unwrap();
        assert_eq!(g.name(), "tmp125");
        // 0°C → register 0 → first byte (high) is 0.
        assert_eq!(g.on_select(), 0x00);
    }

    #[test]
    fn build_tmp125_with_temperature() {
        let arc = build_spi_device("tmp125?temp=25.0").unwrap();
        let mut g = arc.lock().unwrap();
        assert_eq!(g.on_select(), 0x0C);
        assert_eq!(g.on_byte(0x00), 0x80);
    }

    #[test]
    fn build_tmp125_with_negative_temperature() {
        let arc = build_spi_device("tmp125?temp=-10.0").unwrap();
        let mut g = arc.lock().unwrap();
        assert_eq!(g.on_select(), 0x7B);
        assert_eq!(g.on_byte(0x00), 0x00);
    }

    #[test]
    fn build_tmp125_unknown_param_rejected() {
        let err = expect_err("tmp125?wrap=10");
        assert!(err.contains("unknown tmp125 param"), "got: {err}");
    }

    #[test]
    fn build_unknown_device_rejected() {
        let err = expect_err("frobnicator");
        assert!(err.contains("frobnicator"), "got: {err}");
    }

    #[test]
    fn build_unknown_param_rejected() {
        let err = expect_err("echo?wrap=10");
        assert!(err.contains("unknown echo param"), "got: {err}");
    }
}
