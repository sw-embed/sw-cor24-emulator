//! COR24 Emulator - runtime for the C-Oriented RISC 24-bit architecture.
//!
//! - 3 general-purpose 24-bit registers (r0, r1, r2)
//! - 5 special registers: fp=r3, sp=r4, z=r5, iv=r6, ir=r7
//! - Single condition flag (C)
//! - Variable-length instructions (1, 2, or 4 bytes)
//! - 16MB address space (24-bit)
//! - Little-endian byte ordering
//!
//! Assembly (`.s` -> `.lgo` / `.bin` / `.lst`) is the job of `cor24-asm`
//! in the `sw-cor24-x-assembler` crate. This crate is runtime-only:
//! it consumes pre-built artifacts via [`load_lgo`] and friends.

pub mod cpu;
pub mod emulator;
pub mod loader;
pub mod peripherals;

// Re-export main types for convenience
pub use cpu::{
    CpuState, DecodeRom, ExecuteResult, Executor, INITIAL_SP, MEMORY_SIZE, RESET_ADDRESS,
    UartDirection, UartLog, UartLogEntry,
};
pub use emulator::{BatchResult, CpuSnapshot, EmulatorCore, StopReason};
pub use loader::{LoadResult, load_lgo};
