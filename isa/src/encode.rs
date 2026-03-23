//! COR24 instruction encoding tables.
//!
//! Auto-generated from dis_rom.v — canonical (opcode, ra, rb) → byte mapping.

use crate::opcode::Opcode;

/// Encode instruction to byte. Returns None if encoding not found.
pub fn encode_instruction(opcode: Opcode, ra: u8, rb: u8) -> Option<u8> {
    let ra = ra & 0x07;
    let rb = rb & 0x07;

    match opcode {
        Opcode::AddReg => match (ra, rb) {
            (0, 0) => Some(0x00),
            (0, 1) => Some(0x01),
            (0, 2) => Some(0x02),
            (0, 3) => Some(0x58),
            (1, 0) => Some(0x03),
            (1, 1) => Some(0x04),
            (1, 2) => Some(0x05),
            (1, 3) => Some(0x5C),
            (2, 0) => Some(0x06),
            (2, 1) => Some(0x07),
            (2, 2) => Some(0x08),
            (2, 3) => Some(0x60),
            _ => None,
        },
        Opcode::AddImm => match (ra, rb) {
            (0, 7) => Some(0x09),
            (1, 7) => Some(0x0A),
            (2, 7) => Some(0x0B),
            (4, 7) => Some(0x0C),
            _ => None,
        },
        Opcode::And => match (ra, rb) {
            (0, 1) => Some(0x0D),
            (0, 2) => Some(0x0E),
            (1, 0) => Some(0x0F),
            (1, 2) => Some(0x10),
            (2, 0) => Some(0x11),
            (2, 1) => Some(0x12),
            _ => None,
        },
        Opcode::Bra => match (ra, rb) {
            (7, 7) => Some(0x13),
            _ => None,
        },
        Opcode::Brf => match (ra, rb) {
            (7, 7) => Some(0x14),
            _ => None,
        },
        Opcode::Brt => match (ra, rb) {
            (7, 7) => Some(0x15),
            _ => None,
        },
        Opcode::Ceq => match (ra, rb) {
            (0, 1) => Some(0x16),
            (0, 2) => Some(0x17),
            (0, 5) => Some(0xC8),
            (1, 2) => Some(0x18),
            (1, 5) => Some(0xC9),
            (2, 5) => Some(0xCA),
            _ => None,
        },
        Opcode::Cls => match (ra, rb) {
            (0, 1) => Some(0x19),
            (0, 2) => Some(0x1A),
            (0, 5) => Some(0xCB),
            (1, 0) => Some(0x1B),
            (1, 2) => Some(0x1C),
            (1, 5) => Some(0xCC),
            (2, 0) => Some(0x1D),
            (2, 1) => Some(0x1E),
            (2, 5) => Some(0xCD),
            _ => None,
        },
        Opcode::Clu => match (ra, rb) {
            (0, 1) => Some(0x1F),
            (0, 2) => Some(0x20),
            (1, 0) => Some(0x21),
            (1, 2) => Some(0x22),
            (2, 0) => Some(0x23),
            (2, 1) => Some(0x24),
            (5, 0) => Some(0xCE),
            (5, 1) => Some(0xCF),
            (5, 2) => Some(0xD0),
            _ => None,
        },
        Opcode::Jal => match (ra, rb) {
            (1, 0) => Some(0x25),
            (1, 1) => Some(0xD1),
            (1, 2) => Some(0xD2),
            _ => None,
        },
        Opcode::Jmp => match (ra, rb) {
            (0, 7) => Some(0x26),
            (1, 7) => Some(0x27),
            (2, 7) => Some(0x28),
            (7, 7) => Some(0x68),
            _ => None,
        },
        Opcode::La => match (ra, rb) {
            (0, 7) => Some(0x29),
            (1, 7) => Some(0x2A),
            (2, 7) => Some(0x2B),
            (7, 7) => Some(0xC7),
            _ => None,
        },
        Opcode::Lb => match (ra, rb) {
            (0, 0) => Some(0x2C),
            (0, 1) => Some(0x2D),
            (0, 2) => Some(0x2E),
            (0, 3) => Some(0x2F),
            (1, 0) => Some(0x30),
            (1, 1) => Some(0x31),
            (1, 2) => Some(0x32),
            (1, 3) => Some(0x33),
            (2, 0) => Some(0x34),
            (2, 1) => Some(0x35),
            (2, 2) => Some(0x36),
            (2, 3) => Some(0x37),
            _ => None,
        },
        Opcode::Lbu => match (ra, rb) {
            (0, 0) => Some(0x38),
            (0, 1) => Some(0x39),
            (0, 2) => Some(0x3A),
            (0, 3) => Some(0x3B),
            (1, 0) => Some(0x3C),
            (1, 1) => Some(0x3D),
            (1, 2) => Some(0x3E),
            (1, 3) => Some(0x3F),
            (2, 0) => Some(0x40),
            (2, 1) => Some(0x41),
            (2, 2) => Some(0x42),
            (2, 3) => Some(0x43),
            _ => None,
        },
        Opcode::Lc => match (ra, rb) {
            (0, 7) => Some(0x44),
            (1, 7) => Some(0x45),
            (2, 7) => Some(0x46),
            _ => None,
        },
        Opcode::Lcu => match (ra, rb) {
            (0, 7) => Some(0x47),
            (1, 7) => Some(0x48),
            (2, 7) => Some(0x49),
            _ => None,
        },
        Opcode::Lw => match (ra, rb) {
            (0, 0) => Some(0x4A),
            (0, 1) => Some(0x4B),
            (0, 2) => Some(0x4C),
            (0, 3) => Some(0x4D),
            (1, 0) => Some(0x4E),
            (1, 1) => Some(0x4F),
            (1, 2) => Some(0x50),
            (1, 3) => Some(0x51),
            (2, 0) => Some(0x52),
            (2, 1) => Some(0x53),
            (2, 2) => Some(0x54),
            (2, 3) => Some(0x55),
            _ => None,
        },
        Opcode::Mov => match (ra, rb) {
            (0, 1) => Some(0x56),
            (0, 2) => Some(0x57),
            (0, 4) => Some(0x59),
            (0, 5) => Some(0x62),
            (1, 0) => Some(0x5A),
            (1, 2) => Some(0x5B),
            (1, 4) => Some(0x5D),
            (1, 5) => Some(0x63),
            (2, 0) => Some(0x5E),
            (2, 1) => Some(0x5F),
            (2, 4) => Some(0x61),
            (2, 5) => Some(0x64),
            (3, 4) => Some(0x65),
            (4, 0) => Some(0x66),
            (4, 3) => Some(0x69),
            (6, 0) => Some(0x67),
            _ => None,
        },
        Opcode::Mul => match (ra, rb) {
            (0, 0) => Some(0x6A),
            (0, 1) => Some(0x6B),
            (0, 2) => Some(0x6C),
            (1, 0) => Some(0x6D),
            (1, 1) => Some(0x6E),
            (1, 2) => Some(0x6F),
            (2, 0) => Some(0x70),
            (2, 1) => Some(0x71),
            (2, 2) => Some(0x72),
            _ => None,
        },
        Opcode::Or => match (ra, rb) {
            (0, 1) => Some(0x73),
            (0, 2) => Some(0x74),
            (1, 0) => Some(0x75),
            (1, 2) => Some(0x76),
            (2, 0) => Some(0x77),
            (2, 1) => Some(0x78),
            _ => None,
        },
        Opcode::Pop => match (ra, rb) {
            (0, 4) => Some(0x79),
            (1, 4) => Some(0x7A),
            (2, 4) => Some(0x7B),
            (3, 4) => Some(0x7C),
            _ => None,
        },
        Opcode::Push => match (ra, rb) {
            (0, 4) => Some(0x7D),
            (1, 4) => Some(0x7E),
            (2, 4) => Some(0x7F),
            (3, 4) => Some(0x80),
            _ => None,
        },
        Opcode::Sb => match (ra, rb) {
            (0, 1) => Some(0x81),
            (0, 2) => Some(0x82),
            (0, 3) => Some(0x83),
            (1, 0) => Some(0x84),
            (1, 2) => Some(0x85),
            (1, 3) => Some(0x86),
            (2, 0) => Some(0x87),
            (2, 1) => Some(0x88),
            (2, 3) => Some(0x89),
            _ => None,
        },
        Opcode::Shl => match (ra, rb) {
            (0, 1) => Some(0x8A),
            (0, 2) => Some(0x8B),
            (1, 0) => Some(0x8C),
            (1, 2) => Some(0x8D),
            (2, 0) => Some(0x8E),
            (2, 1) => Some(0x8F),
            _ => None,
        },
        Opcode::Sra => match (ra, rb) {
            (0, 1) => Some(0x90),
            (0, 2) => Some(0x91),
            (1, 0) => Some(0x92),
            (1, 2) => Some(0x93),
            (2, 0) => Some(0x94),
            (2, 1) => Some(0x95),
            _ => None,
        },
        Opcode::Srl => match (ra, rb) {
            (0, 1) => Some(0x96),
            (0, 2) => Some(0x97),
            (1, 0) => Some(0x98),
            (1, 2) => Some(0x99),
            (2, 0) => Some(0x9A),
            (2, 1) => Some(0x9B),
            _ => None,
        },
        Opcode::Sub => match (ra, rb) {
            (0, 1) => Some(0x9C),
            (0, 2) => Some(0x9D),
            (1, 0) => Some(0x9E),
            (1, 2) => Some(0x9F),
            (2, 0) => Some(0xA0),
            (2, 1) => Some(0xA1),
            _ => None,
        },
        Opcode::SubSp => match (ra, rb) {
            (4, 7) => Some(0xA2),
            _ => None,
        },
        Opcode::Sw => match (ra, rb) {
            (0, 0) => Some(0xA3),
            (0, 1) => Some(0xA4),
            (0, 2) => Some(0xA5),
            (0, 3) => Some(0xA6),
            (1, 0) => Some(0xA7),
            (1, 1) => Some(0xA8),
            (1, 2) => Some(0xA9),
            (1, 3) => Some(0xAA),
            (2, 0) => Some(0xAB),
            (2, 1) => Some(0xAC),
            (2, 2) => Some(0xAD),
            (2, 3) => Some(0xAE),
            _ => None,
        },
        Opcode::Sxt => match (ra, rb) {
            (0, 0) => Some(0xAF),
            (0, 1) => Some(0xB0),
            (0, 2) => Some(0xB1),
            (1, 0) => Some(0xB2),
            (1, 1) => Some(0xB3),
            (1, 2) => Some(0xB4),
            (2, 0) => Some(0xB5),
            (2, 1) => Some(0xB6),
            (2, 2) => Some(0xB7),
            _ => None,
        },
        Opcode::Xor => match (ra, rb) {
            (0, 1) => Some(0xB8),
            (0, 2) => Some(0xB9),
            (1, 0) => Some(0xBA),
            (1, 2) => Some(0xBB),
            (2, 0) => Some(0xBC),
            (2, 1) => Some(0xBD),
            _ => None,
        },
        Opcode::Zxt => match (ra, rb) {
            (0, 0) => Some(0xBE),
            (0, 1) => Some(0xBF),
            (0, 2) => Some(0xC0),
            (1, 0) => Some(0xC1),
            (1, 1) => Some(0xC2),
            (1, 2) => Some(0xC3),
            (2, 0) => Some(0xC4),
            (2, 1) => Some(0xC5),
            (2, 2) => Some(0xC6),
            _ => None,
        },
        Opcode::Invalid => None,
    }
}

/// Encode branch instruction first byte.
pub fn encode_branch(opcode: Opcode) -> Option<u8> {
    encode_instruction(opcode, 7, 7)
}

/// Encode add register: add ra,rb
pub fn encode_add_reg(ra: u8, rb: u8) -> Option<u8> {
    encode_instruction(Opcode::AddReg, ra, rb)
}

/// Encode add immediate first byte: add ra,imm8
pub fn encode_add_imm(ra: u8) -> Option<u8> {
    encode_instruction(Opcode::AddImm, ra, 7)
}

/// Encode mov: mov ra,rb
pub fn encode_mov(ra: u8, rb: u8) -> Option<u8> {
    encode_instruction(Opcode::Mov, ra, rb)
}

/// Encode push: push ra
pub fn encode_push(ra: u8) -> Option<u8> {
    encode_instruction(Opcode::Push, ra, 4)
}

/// Encode pop: pop ra
pub fn encode_pop(ra: u8) -> Option<u8> {
    encode_instruction(Opcode::Pop, ra, 4)
}

/// Encode load/store first byte: lb/lbu/lw/sb/sw ra,offset(rb)
pub fn encode_load_store(opcode: Opcode, ra: u8, rb: u8) -> Option<u8> {
    encode_instruction(opcode, ra, rb)
}

/// Encode lc/lcu first byte: lc ra,imm8
pub fn encode_lc(ra: u8, unsigned: bool) -> Option<u8> {
    let opcode = if unsigned { Opcode::Lcu } else { Opcode::Lc };
    encode_instruction(opcode, ra, 7)
}

/// Encode la first byte: la ra,addr24
pub fn encode_la(ra: u8) -> Option<u8> {
    encode_instruction(Opcode::La, ra, 7)
}

/// Encode sub sp,imm24 first byte
pub fn encode_sub_sp() -> Option<u8> {
    encode_instruction(Opcode::SubSp, 4, 7)
}

/// Encode jmp (ra)
pub fn encode_jmp(ra: u8) -> Option<u8> {
    encode_instruction(Opcode::Jmp, ra, 7)
}

/// Encode jal ra,(rb)
pub fn encode_jal(ra: u8, rb: u8) -> Option<u8> {
    encode_instruction(Opcode::Jal, ra, rb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_branch() {
        assert_eq!(encode_branch(Opcode::Bra), Some(0x13));
        assert_eq!(encode_branch(Opcode::Brf), Some(0x14));
        assert_eq!(encode_branch(Opcode::Brt), Some(0x15));
    }

    #[test]
    fn test_encode_push_pop() {
        assert_eq!(encode_push(0), Some(0x7D));
        assert_eq!(encode_push(1), Some(0x7E));
        assert_eq!(encode_pop(0), Some(0x79));
        assert_eq!(encode_pop(1), Some(0x7A));
    }

    #[test]
    fn test_encode_mov() {
        assert_eq!(encode_mov(3, 4), Some(0x65)); // mov fp,sp
        assert_eq!(encode_mov(4, 3), Some(0x69)); // mov sp,fp
    }
}
