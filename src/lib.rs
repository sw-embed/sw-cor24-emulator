//! COR24 Assembly Emulator - Educational Programming Tool
//!
//! A COR24 emulator that teaches COR24 assembly programming through
//! interactive examples and challenges.
//!
//! COR24 is a C-Oriented RISC 24-bit architecture with:
//! - 3 general-purpose 24-bit registers (r0, r1, r2)
//! - 5 special registers: fp=r3, sp=r4, z=r5, iv=r6, ir=r7
//! - Single condition flag (C)
//! - Variable-length instructions (1, 2, or 4 bytes)
//! - 16MB address space (24-bit)
//! - Little-endian byte ordering

pub mod assembler;
pub mod challenge;
pub mod cpu;
pub mod emulator;
pub mod loader;

// Re-export main types for convenience
pub use assembler::{AssembledLine, Assembler, AssemblyResult};
pub use challenge::{Challenge, get_challenges, get_examples};
pub use cpu::{
    CpuState, DecodeRom, ExecuteResult, Executor, INITIAL_SP, MEMORY_SIZE, RESET_ADDRESS,
};
pub use emulator::{BatchResult, CpuSnapshot, EmulatorCore, StopReason};
pub use loader::{LoadResult, load_lgo};
