//! CLI device registry for SPI.
//!
//! `build_spi_device` is the string-keyed registry the CLI parses. SPI
//! is single-slave today (plan §9: future multi-slave SELN bitmask).
//! Spec syntax: `<name>[@cs=<n>][?key=val&...]`. The `@cs=<n>` form is
//! parsed and stored on the device for observation but not yet
//! enforced — multi-slave routing lands later. Older specs without
//! `@cs=` continue to parse.
//!
//! Recognised devices:
//!   - `echo[?seed=<n>]`            — universal echo test slave.
//!   - `tmp125[?temp=<f>]`          — TI TMP125 temperature sensor.
//!   - `sdcard[@cs=<n>][?file=<path>]` — SD card in SPI mode.
//!   - `w25q32[@cs=<n>][?file=<path>]` — Winbond W25Q32 NOR flash (4 MiB).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::device::SpiDevice;
use super::devices::echo::EchoDevice;
use super::devices::sdcard::{DEFAULT_CS as SDCARD_DEFAULT_CS, SdCardDevice};
use super::devices::tmp125::Tmp125Device;
use super::devices::w25q32::{DEFAULT_CS as W25Q32_DEFAULT_CS, W25q32Device};

pub fn build_spi_device(spec: &str) -> Result<Arc<Mutex<dyn SpiDevice>>, String> {
    let (head, params) = match spec.split_once('?') {
        Some((h, t)) => (h, Some(t)),
        None => (spec, None),
    };
    // `@cs=<n>` is forward-compat universal — accepted for every device
    // and silently honored when the underlying device tracks a CS pin
    // (sdcard, w25q32). Devices without their own CS field (echo,
    // tmp125) accept the syntax and ignore the value; when multi-slave
    // SPI lands in plan §9 they pick it up uniformly.
    let (name, cs_override) = split_cs(head, spec)?;
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
        "sdcard" => {
            let cs = cs_override.unwrap_or(SDCARD_DEFAULT_CS);
            let mut file: Option<PathBuf> = None;
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "file" => file = Some(PathBuf::from(v)),
                        _ => return Err(format!("unknown sdcard param '{k}' in '{spec}'")),
                    }
                }
            }
            let dev = if let Some(path) = file {
                SdCardDevice::from_file(&path, cs)
                    .map_err(|e| format!("sdcard file '{}': {e}", path.display()))?
            } else {
                let mut d = SdCardDevice::new();
                // Apply CS override to the in-memory scratch device.
                if cs_override.is_some() {
                    d = SdCardDevice::with_image(d.image(), None, cs);
                }
                d
            };
            Ok(Arc::new(Mutex::new(dev)))
        }
        "w25q32" => {
            let cs = cs_override.unwrap_or(W25Q32_DEFAULT_CS);
            let mut file: Option<PathBuf> = None;
            if let Some(p) = params {
                for kv in p.split('&') {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad param '{kv}' in '{spec}'"))?;
                    match k {
                        "file" => file = Some(PathBuf::from(v)),
                        _ => return Err(format!("unknown w25q32 param '{k}' in '{spec}'")),
                    }
                }
            }
            let dev = if let Some(path) = file {
                W25q32Device::from_file(&path, cs)
                    .map_err(|e| format!("w25q32 file '{}': {e}", path.display()))?
            } else {
                W25q32Device::with_image(vec![0xFF; super::devices::w25q32::IMAGE_SIZE], None, cs)
            };
            Ok(Arc::new(Mutex::new(dev)))
        }
        other => Err(format!("unknown SPI device '{other}'")),
    }
}

/// Split the head (before `?`) into `(name, cs_override)`. The head
/// form is `<name>[@cs=<n>]`. Examples: `sdcard` → ("sdcard", None);
/// `sdcard@cs=2` → ("sdcard", Some(2)).
fn split_cs<'a>(head: &'a str, spec: &str) -> Result<(&'a str, Option<u8>), String> {
    match head.split_once('@') {
        None => Ok((head, None)),
        Some((name, rest)) => {
            let cs_str = rest
                .strip_prefix("cs=")
                .ok_or_else(|| format!("bad '@' qualifier '@{rest}' in '{spec}' (expected '@cs=<n>')"))?;
            let cs = parse_u8(cs_str)
                .ok_or_else(|| format!("bad cs '{cs_str}' in '{spec}'"))?;
            Ok((name, Some(cs)))
        }
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

    #[test]
    fn build_sdcard_default() {
        let arc = build_spi_device("sdcard").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "sdcard");
    }

    #[test]
    fn build_sdcard_with_cs() {
        let arc = build_spi_device("sdcard@cs=2").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "sdcard");
    }

    #[test]
    fn build_sdcard_unknown_param_rejected() {
        let err = expect_err("sdcard?temp=25.0");
        assert!(err.contains("unknown sdcard param"), "got: {err}");
    }

    #[test]
    fn build_sdcard_bad_cs_rejected() {
        let err = expect_err("sdcard@cs=oops");
        assert!(err.contains("bad cs"), "got: {err}");
    }

    #[test]
    fn build_sdcard_missing_file_rejected() {
        let err = expect_err("sdcard?file=/nonexistent/path/that/should/never/exist");
        assert!(err.contains("sdcard file"), "got: {err}");
    }

    #[test]
    fn echo_accepts_at_cs_for_forward_compat() {
        // The @cs=<n> syntax is universal across SPI devices for
        // forward-compat with plan §9 multi-slave routing. Devices
        // that don't yet model a CS field accept it and ignore.
        let arc = build_spi_device("echo@cs=1").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "echo");
    }

    #[test]
    fn tmp125_accepts_at_cs_for_forward_compat() {
        let arc = build_spi_device("tmp125@cs=1?temp=25.0").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "tmp125");
    }

    #[test]
    fn build_w25q32_default() {
        let arc = build_spi_device("w25q32").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "w25q32");
    }

    #[test]
    fn build_w25q32_with_cs() {
        let arc = build_spi_device("w25q32@cs=3").unwrap();
        let g = arc.lock().unwrap();
        assert_eq!(g.name(), "w25q32");
    }

    #[test]
    fn build_w25q32_unknown_param_rejected() {
        let err = expect_err("w25q32?temp=25.0");
        assert!(err.contains("unknown w25q32 param"), "got: {err}");
    }

    #[test]
    fn build_w25q32_missing_file_rejected() {
        let err = expect_err("w25q32?file=/nonexistent/path/that/should/never/exist");
        assert!(err.contains("w25q32 file"), "got: {err}");
    }
}
