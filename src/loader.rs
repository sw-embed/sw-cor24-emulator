//! LGO file loader for COR24
//!
//! Parses the monitor "load and go" format produced by `as24 | longlgo`.
//!
//! Format:
//!   L<AAAAAA><HH><HH>...   Load hex bytes at 24-bit address
//!   G<AAAAAA>               Go — set PC to address (optional)
//!
//! The address is 6 hex digits. Data bytes follow as pairs of hex digits.

use crate::cpu::state::CpuState;

/// Result of loading an LGO file
pub struct LoadResult {
    /// Start address from G line, if present
    pub start_addr: Option<u32>,
    /// Total bytes loaded
    pub bytes_loaded: usize,
}

/// Parse a single LGO L-line, returning (address, bytes)
pub fn parse_lgo_load_line(line: &str) -> Result<(u32, Vec<u8>), String> {
    let line = line.trim();
    if !line.starts_with('L') || line.len() < 7 {
        return Err(format!("Not a valid L line: '{}'", line));
    }

    let addr_str = &line[1..7];
    let addr = u32::from_str_radix(addr_str, 16)
        .map_err(|e| format!("Bad address '{}': {}", addr_str, e))?;

    let data_str = &line[7..];
    if data_str.len() % 2 != 0 {
        return Err(format!("Odd number of hex digits in data: '{}'", data_str));
    }

    let mut bytes = Vec::with_capacity(data_str.len() / 2);
    for i in (0..data_str.len()).step_by(2) {
        let byte = u8::from_str_radix(&data_str[i..i + 2], 16)
            .map_err(|e| format!("Bad hex byte '{}': {}", &data_str[i..i + 2], e))?;
        bytes.push(byte);
    }

    Ok((addr, bytes))
}

/// Parse a G-line, returning the start address
pub fn parse_lgo_go_line(line: &str) -> Result<u32, String> {
    let line = line.trim();
    if !line.starts_with('G') || line.len() < 7 {
        return Err(format!("Not a valid G line: '{}'", line));
    }

    let addr_str = &line[1..7];
    u32::from_str_radix(addr_str, 16)
        .map_err(|e| format!("Bad address '{}': {}", addr_str, e))
}

/// Load an LGO file into CPU memory
pub fn load_lgo(content: &str, cpu: &mut CpuState) -> Result<LoadResult, String> {
    let mut start_addr = None;
    let mut bytes_loaded = 0usize;

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line.chars().next() {
            Some('L') => {
                let (addr, bytes) = parse_lgo_load_line(line)
                    .map_err(|e| format!("Line {}: {}", line_num + 1, e))?;
                for (i, &byte) in bytes.iter().enumerate() {
                    cpu.write_byte(addr + i as u32, byte);
                }
                bytes_loaded += bytes.len();
            }
            Some('G') => {
                start_addr = Some(
                    parse_lgo_go_line(line)
                        .map_err(|e| format!("Line {}: {}", line_num + 1, e))?,
                );
            }
            Some(';') | Some('#') => {
                // Comment line, skip
            }
            _ => {
                return Err(format!("Line {}: Unknown line type: '{}'", line_num + 1, line));
            }
        }
    }

    Ok(LoadResult {
        start_addr,
        bytes_loaded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lgo_load_line_simple() {
        let (addr, bytes) = parse_lgo_load_line("L00000080").unwrap();
        assert_eq!(addr, 0x000000);
        assert_eq!(bytes, vec![0x80]);
    }

    #[test]
    fn test_parse_lgo_load_line_multi_byte() {
        let (addr, bytes) = parse_lgo_load_line("L0000042B0001FF").unwrap();
        assert_eq!(addr, 0x000004);
        assert_eq!(bytes, vec![0x2B, 0x00, 0x01, 0xFF]);
    }

    #[test]
    fn test_parse_lgo_load_line_long() {
        // First line of sieve.lgo
        let line = "L000000807F7E652B0001FF2E0145020DCE14F62E01CB15F92F098200697A7B7C27807F7E650CF7";
        let (addr, bytes) = parse_lgo_load_line(line).unwrap();
        assert_eq!(addr, 0x000000);
        assert_eq!(bytes[0], 0x80); // push fp
        assert_eq!(bytes[1], 0x7F); // push r2
        assert_eq!(bytes[2], 0x7E); // push r1
        assert_eq!(bytes[3], 0x65); // mov fp,sp
    }

    #[test]
    fn test_parse_lgo_load_line_high_addr() {
        let (addr, bytes) = parse_lgo_load_line("L00213E31303030").unwrap();
        assert_eq!(addr, 0x00213E);
        assert_eq!(bytes, vec![0x31, 0x30, 0x30, 0x30]);
    }

    #[test]
    fn test_parse_lgo_go_line() {
        let addr = parse_lgo_go_line("G000093").unwrap();
        assert_eq!(addr, 0x000093);
    }

    #[test]
    fn test_parse_lgo_go_line_zero() {
        let addr = parse_lgo_go_line("G000000").unwrap();
        assert_eq!(addr, 0x000000);
    }

    #[test]
    fn test_parse_lgo_bad_line() {
        assert!(parse_lgo_load_line("X000000FF").is_err());
        assert!(parse_lgo_load_line("L0000").is_err()); // too short
        assert!(parse_lgo_load_line("L000000F").is_err()); // odd hex digits
    }

    #[test]
    fn test_load_lgo_into_cpu() {
        let lgo = "L00000080\nL0000017F\n";
        let mut cpu = CpuState::new();
        let result = load_lgo(lgo, &mut cpu).unwrap();
        assert_eq!(result.bytes_loaded, 2);
        assert_eq!(result.start_addr, None);
        assert_eq!(cpu.read_byte(0x000000), 0x80); // push fp
        assert_eq!(cpu.read_byte(0x000001), 0x7F); // push r2
    }

    #[test]
    fn test_load_lgo_with_go() {
        let lgo = "L00000080\nG000093\n";
        let mut cpu = CpuState::new();
        let result = load_lgo(lgo, &mut cpu).unwrap();
        assert_eq!(result.start_addr, Some(0x000093));
    }

    #[test]
    fn test_load_lgo_empty_lines_and_comments() {
        let lgo = "; comment\nL00000080\n\n# another comment\n";
        let mut cpu = CpuState::new();
        let result = load_lgo(lgo, &mut cpu).unwrap();
        assert_eq!(result.bytes_loaded, 1);
    }

    #[test]
    fn test_load_sieve_lgo() {
        let lgo = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/docs/research/asld24/sieve.lgo"),
        )
        .expect("sieve.lgo must exist");

        let mut cpu = CpuState::new();
        let result = load_lgo(&lgo, &mut cpu).unwrap();

        // First instruction: push fp = 0x80
        assert_eq!(cpu.read_byte(0x000000), 0x80, "First byte should be push fp");

        // _main entry at 0x93: push fp = 0x80
        assert_eq!(cpu.read_byte(0x000093), 0x80, "_main should start with push fp");

        // Data section: "1000 iterations\n" at 0x213E
        assert_eq!(cpu.read_byte(0x00213E), 0x31, "Data '1' at 0x213E");
        assert_eq!(cpu.read_byte(0x00213F), 0x30, "Data '0' at 0x213F");

        // Should have loaded a good chunk of bytes
        assert!(result.bytes_loaded > 300, "Should load >300 bytes, got {}", result.bytes_loaded);
    }
}
