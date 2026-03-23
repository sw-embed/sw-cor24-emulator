//! COR24 instruction opcodes and formats.

/// COR24 instruction opcodes (5-bit, from decoded ROM)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Opcode {
    AddReg = 0x00, // add ra,rb
    AddImm = 0x01, // add ra,dd
    And = 0x02,    // and ra,rb
    Bra = 0x03,    // bra dd
    Brf = 0x04,    // brf dd
    Brt = 0x05,    // brt dd
    Ceq = 0x06,    // ceq ra,rb
    Cls = 0x07,    // cls ra,rb (compare less signed)
    Clu = 0x08,    // clu ra,rb (compare less unsigned)
    Jal = 0x09,    // jal ra,(rb)
    Jmp = 0x0A,    // jmp (ra)
    La = 0x0B,     // la ra,dddddd
    Lb = 0x0C,     // lb ra,dd(rb) (signed)
    Lbu = 0x0D,    // lbu ra,dd(rb) (unsigned)
    Lc = 0x0E,     // lc ra,dd (signed)
    Lcu = 0x0F,    // lcu ra,dd (unsigned)
    Lw = 0x10,     // lw ra,dd(rb)
    Mov = 0x11,    // mov ra,rb or mov ra,c
    Mul = 0x12,    // mul ra,rb
    Or = 0x13,     // or ra,rb
    Pop = 0x14,    // pop ra
    Push = 0x15,   // push ra
    Sb = 0x16,     // sb ra,dd(rb)
    Shl = 0x17,    // shl ra,rb
    Sra = 0x18,    // sra ra,rb
    Srl = 0x19,    // srl ra,rb
    Sub = 0x1A,    // sub ra,rb
    SubSp = 0x1B,  // sub sp,dddddd
    Sw = 0x1C,     // sw ra,dd(rb)
    Sxt = 0x1D,    // sxt ra,rb
    Xor = 0x1E,    // xor ra,rb
    Zxt = 0x1F,    // zxt ra,rb
    Invalid = 0xFF,
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Opcode::AddReg,
            0x01 => Opcode::AddImm,
            0x02 => Opcode::And,
            0x03 => Opcode::Bra,
            0x04 => Opcode::Brf,
            0x05 => Opcode::Brt,
            0x06 => Opcode::Ceq,
            0x07 => Opcode::Cls,
            0x08 => Opcode::Clu,
            0x09 => Opcode::Jal,
            0x0A => Opcode::Jmp,
            0x0B => Opcode::La,
            0x0C => Opcode::Lb,
            0x0D => Opcode::Lbu,
            0x0E => Opcode::Lc,
            0x0F => Opcode::Lcu,
            0x10 => Opcode::Lw,
            0x11 => Opcode::Mov,
            0x12 => Opcode::Mul,
            0x13 => Opcode::Or,
            0x14 => Opcode::Pop,
            0x15 => Opcode::Push,
            0x16 => Opcode::Sb,
            0x17 => Opcode::Shl,
            0x18 => Opcode::Sra,
            0x19 => Opcode::Srl,
            0x1A => Opcode::Sub,
            0x1B => Opcode::SubSp,
            0x1C => Opcode::Sw,
            0x1D => Opcode::Sxt,
            0x1E => Opcode::Xor,
            0x1F => Opcode::Zxt,
            _ => Opcode::Invalid,
        }
    }
}

/// Decoded instruction with opcode and register operands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub opcode: Opcode,
    pub ra: u8,
    pub rb: u8,
}

impl DecodedInstruction {
    pub fn new(opcode: Opcode, ra: u8, rb: u8) -> Self {
        Self {
            opcode,
            ra: ra & 0x07,
            rb: rb & 0x07,
        }
    }

    /// Decode from 12-bit ROM output
    pub fn from_decoded(decoded: u16) -> Self {
        let opcode = Opcode::from(((decoded >> 6) & 0x1F) as u8);
        let ra = ((decoded >> 3) & 0x07) as u8;
        let rb = (decoded & 0x07) as u8;
        Self { opcode, ra, rb }
    }
}

/// Instruction format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionFormat {
    /// Single byte: opcode + registers encoded
    SingleByte,
    /// Two bytes: first byte + 8-bit immediate/offset
    TwoBytes,
    /// Four bytes: first byte + 24-bit address
    FourBytes,
}

impl InstructionFormat {
    /// Size in bytes
    pub fn size(&self) -> usize {
        match self {
            InstructionFormat::SingleByte => 1,
            InstructionFormat::TwoBytes => 2,
            InstructionFormat::FourBytes => 4,
        }
    }
}

impl Opcode {
    /// Get the instruction format for this opcode
    pub fn format(&self) -> InstructionFormat {
        match self {
            Opcode::AddReg | Opcode::And | Opcode::Ceq | Opcode::Cls | Opcode::Clu
            | Opcode::Jal | Opcode::Jmp | Opcode::Mov | Opcode::Mul | Opcode::Or
            | Opcode::Pop | Opcode::Push | Opcode::Shl | Opcode::Sra | Opcode::Srl
            | Opcode::Sub | Opcode::Sxt | Opcode::Xor | Opcode::Zxt => {
                InstructionFormat::SingleByte
            }
            Opcode::AddImm | Opcode::Bra | Opcode::Brf | Opcode::Brt | Opcode::Lb
            | Opcode::Lbu | Opcode::Lc | Opcode::Lcu | Opcode::Lw | Opcode::Sb
            | Opcode::Sw => InstructionFormat::TwoBytes,
            Opcode::La | Opcode::SubSp => InstructionFormat::FourBytes,
            Opcode::Invalid => InstructionFormat::SingleByte,
        }
    }

    /// Get mnemonic string for this opcode
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Opcode::AddReg | Opcode::AddImm => "add",
            Opcode::And => "and",
            Opcode::Bra => "bra",
            Opcode::Brf => "brf",
            Opcode::Brt => "brt",
            Opcode::Ceq => "ceq",
            Opcode::Cls => "cls",
            Opcode::Clu => "clu",
            Opcode::Jal => "jal",
            Opcode::Jmp => "jmp",
            Opcode::La => "la",
            Opcode::Lb => "lb",
            Opcode::Lbu => "lbu",
            Opcode::Lc => "lc",
            Opcode::Lcu => "lcu",
            Opcode::Lw => "lw",
            Opcode::Mov => "mov",
            Opcode::Mul => "mul",
            Opcode::Or => "or",
            Opcode::Pop => "pop",
            Opcode::Push => "push",
            Opcode::Sb => "sb",
            Opcode::Shl => "shl",
            Opcode::Sra => "sra",
            Opcode::Srl => "srl",
            Opcode::Sub | Opcode::SubSp => "sub",
            Opcode::Sw => "sw",
            Opcode::Sxt => "sxt",
            Opcode::Xor => "xor",
            Opcode::Zxt => "zxt",
            Opcode::Invalid => "???",
        }
    }
}
