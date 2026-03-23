//! COR24 ISA definitions shared between compiler and assembler/emulator.
//!
//! This crate defines the canonical instruction set, encoding tables,
//! register names, and branch range constants for the COR24 architecture.

pub mod branch;
pub mod encode;
pub mod opcode;
pub mod register;

pub use branch::{BRANCH_OFFSET_MAX, BRANCH_OFFSET_MIN, BRANCH_PIPELINE_DELAY, MAX_INSTRUCTION_BYTES};
pub use opcode::{DecodedInstruction, InstructionFormat, Opcode};
pub use register::{REG_NAMES, reg_name};
