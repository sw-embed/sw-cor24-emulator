//! COR24 register definitions.

/// Register names indexed by 3-bit register number.
pub const REG_NAMES: [&str; 8] = ["r0", "r1", "r2", "fp", "sp", "z", "r6", "r7"];

/// Get register name from 3-bit index.
pub fn reg_name(reg: u8) -> &'static str {
    REG_NAMES[(reg & 0x07) as usize]
}

/// Parse a register name to its 3-bit index.
pub fn parse_register(s: &str) -> Option<u8> {
    match s.to_lowercase().as_str() {
        "r0" => Some(0),
        "r1" => Some(1),
        "r2" => Some(2),
        "fp" => Some(3),
        "sp" => Some(4),
        "z" | "c" => Some(5),
        "iv" => Some(6),
        "ir" => Some(7),
        _ => None,
    }
}
