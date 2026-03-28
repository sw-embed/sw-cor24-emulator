//! COR24 branch range constants and helpers.
//!
//! Branch instructions (bra, brt, brf) use a 2-byte format:
//!   byte 0: opcode
//!   byte 1: signed 8-bit offset
//!
//! The offset is relative to branch_base = instruction_address + 4
//! (accounting for the COR24 pipeline: 3 prefetch + 1 execute).

/// Minimum branch offset (signed 8-bit).
pub const BRANCH_OFFSET_MIN: i32 = -128;

/// Maximum branch offset (signed 8-bit).
pub const BRANCH_OFFSET_MAX: i32 = 127;

/// Pipeline delay: branch base = instruction_address + this value.
pub const BRANCH_PIPELINE_DELAY: u32 = 4;

/// Maximum bytes a single instruction can occupy.
pub const MAX_INSTRUCTION_BYTES: usize = 4;

/// Maximum number of instructions between a branch and its target
/// that guarantees a short branch is safe.
///
/// Calculated as: BRANCH_OFFSET_MAX / MAX_INSTRUCTION_BYTES = 127 / 4 = 31.
/// This is conservative — most instructions are 1-2 bytes, so the actual
/// reachable instruction count is usually higher.
pub const MAX_SHORT_BRANCH_INSTRUCTIONS: usize = 31;

/// Check if a short branch can reach from one instruction position to another.
///
/// Uses a conservative estimate: each instruction is at most 4 bytes.
/// Returns true if the distance is guaranteed to be within range.
pub fn can_short_branch(from_count: usize, to_count: usize) -> bool {
    from_count.abs_diff(to_count) <= MAX_SHORT_BRANCH_INSTRUCTIONS
}
