//! COR24 CPU emulator
//!
//! The COR24 is a C-Oriented RISC 24-bit architecture with:
//! - 3 general-purpose 24-bit registers (r0, r1, r2)
//! - 5 special registers: fp=r3, sp=r4, z=r5, iv=r6, ir=r7
//! - Single condition flag (C)
//! - Variable-length instructions (1, 2, or 4 bytes)
//! - 16MB address space (24-bit)
//! - Little-endian byte ordering

pub mod decode_rom;
pub mod encode;
pub mod executor;
pub mod instruction;
pub mod state;

pub use decode_rom::DECODE_ROM;
pub use encode::*;
pub use executor::{ExecuteResult, Executor};
pub use instruction::{DecodedInstruction, InstructionFormat, Opcode, REG_NAMES};
pub use state::{CpuState, DecodeRom, INITIAL_SP, MEMORY_SIZE, RESET_ADDRESS, UartDirection, UartLog, UartLogEntry};
