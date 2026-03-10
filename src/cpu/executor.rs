//! COR24 instruction execution

use super::instruction::{DecodedInstruction, InstructionFormat, Opcode};
use super::state::{CpuState, DecodeRom};

/// Execute result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteResult {
    /// Instruction executed successfully
    Ok,
    /// CPU halted
    Halted,
    /// Invalid instruction
    InvalidInstruction(u8),
    /// Memory access error
    MemoryError(u32),
}

/// COR24 instruction executor
#[derive(Clone)]
pub struct Executor {
    decode_rom: DecodeRom,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            decode_rom: DecodeRom::new(),
        }
    }

    /// Execute a single instruction
    pub fn step(&self, cpu: &mut CpuState) -> ExecuteResult {
        if cpu.halted {
            return ExecuteResult::Halted;
        }

        // Fetch instruction byte
        let inst_byte = cpu.read_byte(cpu.pc);

        // Check for HALT (using 0x00 at address 0 as halt)
        if inst_byte == 0x00 && cpu.pc == 0 {
            cpu.halted = true;
            return ExecuteResult::Halted;
        }

        // Decode instruction
        let decoded_value = self.decode_rom.decode(inst_byte);
        if decoded_value == 0xFFF {
            return ExecuteResult::InvalidInstruction(inst_byte);
        }

        let inst = DecodedInstruction::from_decoded(decoded_value);
        let format = inst.opcode.format();

        // Fetch additional bytes based on format
        let imm8 = match format {
            InstructionFormat::TwoBytes | InstructionFormat::FourBytes => {
                cpu.read_byte(cpu.pc.wrapping_add(1))
            }
            _ => 0,
        };

        let imm24 = match format {
            InstructionFormat::FourBytes => {
                let b0 = cpu.read_byte(cpu.pc.wrapping_add(1)) as u32;
                let b1 = cpu.read_byte(cpu.pc.wrapping_add(2)) as u32;
                let b2 = cpu.read_byte(cpu.pc.wrapping_add(3)) as u32;
                b0 | (b1 << 8) | (b2 << 16)
            }
            _ => 0,
        };

        // Calculate next PC
        let inst_size = match format {
            InstructionFormat::SingleByte => 1,
            InstructionFormat::TwoBytes => 2,
            InstructionFormat::FourBytes => 4,
        };
        let next_pc = CpuState::mask_24(cpu.pc.wrapping_add(inst_size));

        // Execute instruction
        match inst.opcode {
            Opcode::AddReg => {
                // add ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                let result = CpuState::mask_24(ra_val.wrapping_add(rb_val));
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::AddImm => {
                // add ra,dd
                let ra_val = cpu.get_reg(inst.ra);
                let imm = CpuState::sign_extend_8(imm8);
                let result = CpuState::mask_24(ra_val.wrapping_add(imm));
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::And => {
                // and ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, ra_val & rb_val);
                cpu.pc = next_pc;
            }

            Opcode::Bra => {
                // bra dd (always branch)
                // Hardware pipeline: PC is 2 ahead of next instruction at execute time
                // So branch base = instruction_addr + 4 = next_pc + 2
                let offset = CpuState::sign_extend_8(imm8);
                let branch_base = next_pc.wrapping_add(2);
                let target = CpuState::mask_24(branch_base.wrapping_add(offset));
                // Detect self-branch (halt: bra halt) as halt condition
                let instruction_addr = CpuState::mask_24(next_pc.wrapping_sub(2));
                if target == instruction_addr {
                    cpu.pc = instruction_addr;
                    cpu.halted = true;
                    return ExecuteResult::Halted;
                }
                cpu.pc = target;
            }

            Opcode::Brf => {
                // brf dd (branch if false)
                if !cpu.c {
                    let offset = CpuState::sign_extend_8(imm8);
                    let branch_base = next_pc.wrapping_add(2);
                    cpu.pc = CpuState::mask_24(branch_base.wrapping_add(offset));
                } else {
                    cpu.pc = next_pc;
                }
            }

            Opcode::Brt => {
                // brt dd (branch if true)
                if cpu.c {
                    let offset = CpuState::sign_extend_8(imm8);
                    let branch_base = next_pc.wrapping_add(2);
                    cpu.pc = CpuState::mask_24(branch_base.wrapping_add(offset));
                } else {
                    cpu.pc = next_pc;
                }
            }

            Opcode::Ceq => {
                // ceq ra,rb (compare equal)
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                cpu.c = ra_val == rb_val;
                cpu.pc = next_pc;
            }

            Opcode::Cls => {
                // cls ra,rb (compare less signed)
                let ra_val = cpu.get_reg(inst.ra) as i32;
                let rb_val = cpu.get_reg(inst.rb) as i32;
                // Sign extend from 24-bit
                let ra_signed = if ra_val & 0x800000 != 0 {
                    ra_val | 0xFF000000u32 as i32
                } else {
                    ra_val
                };
                let rb_signed = if rb_val & 0x800000 != 0 {
                    rb_val | 0xFF000000u32 as i32
                } else {
                    rb_val
                };
                cpu.c = ra_signed < rb_signed;
                cpu.pc = next_pc;
            }

            Opcode::Clu => {
                // clu ra,rb (compare less unsigned)
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                cpu.c = ra_val < rb_val;
                cpu.pc = next_pc;
            }

            Opcode::Jal => {
                // jal ra,(rb) - jump and link
                let target = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, next_pc);
                cpu.pc = target;
            }

            Opcode::Jmp => {
                // jmp (ra)
                let target = cpu.get_reg(inst.ra);
                cpu.pc = target;
            }

            Opcode::La => {
                // la ra,dddddd
                if inst.ra == 7 {
                    // jmp dddddd (absolute jump)
                    cpu.pc = imm24;
                } else {
                    cpu.set_reg(inst.ra, imm24);
                    cpu.pc = next_pc;
                }
            }

            Opcode::Lb => {
                // lb ra,dd(rb) - load byte signed
                let base = cpu.get_reg(inst.rb);
                let offset = CpuState::sign_extend_8(imm8);
                let addr = CpuState::mask_24(base.wrapping_add(offset));
                let value = cpu.read_byte(addr);
                cpu.set_reg(inst.ra, CpuState::sign_extend_8(value));
                cpu.pc = next_pc;
            }

            Opcode::Lbu => {
                // lbu ra,dd(rb) - load byte unsigned
                let base = cpu.get_reg(inst.rb);
                let offset = CpuState::sign_extend_8(imm8);
                let addr = CpuState::mask_24(base.wrapping_add(offset));
                let value = cpu.read_byte(addr);
                cpu.set_reg(inst.ra, value as u32);
                cpu.pc = next_pc;
            }

            Opcode::Lc => {
                // lc ra,dd - load constant signed
                cpu.set_reg(inst.ra, CpuState::sign_extend_8(imm8));
                cpu.pc = next_pc;
            }

            Opcode::Lcu => {
                // lcu ra,dd - load constant unsigned
                cpu.set_reg(inst.ra, imm8 as u32);
                cpu.pc = next_pc;
            }

            Opcode::Lw => {
                // lw ra,dd(rb) - load word
                let base = cpu.get_reg(inst.rb);
                let offset = CpuState::sign_extend_8(imm8);
                let addr = CpuState::mask_24(base.wrapping_add(offset));
                let value = cpu.read_word(addr);
                cpu.set_reg(inst.ra, value);
                cpu.pc = next_pc;
            }

            Opcode::Mov => {
                // mov ra,rb or mov ra,c
                if inst.rb == 5 {
                    // mov ra,c
                    cpu.set_reg(inst.ra, if cpu.c { 1 } else { 0 });
                } else {
                    cpu.set_reg(inst.ra, cpu.get_reg(inst.rb));
                }
                cpu.pc = next_pc;
            }

            Opcode::Mul => {
                // mul ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                let result = CpuState::mask_24(ra_val.wrapping_mul(rb_val));
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::Or => {
                // or ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, ra_val | rb_val);
                cpu.pc = next_pc;
            }

            Opcode::Pop => {
                // pop ra - load from stack, increment sp
                let sp = cpu.get_reg(4);
                let value = cpu.read_word(sp);
                cpu.set_reg(inst.ra, value);
                cpu.set_reg(4, CpuState::mask_24(sp.wrapping_add(3)));
                cpu.pc = next_pc;
            }

            Opcode::Push => {
                // push ra - decrement sp, store to stack
                let sp = cpu.get_reg(4);
                let new_sp = CpuState::mask_24(sp.wrapping_sub(3));
                cpu.set_reg(4, new_sp);
                let value = cpu.get_reg(inst.ra);
                cpu.write_word(new_sp, value);
                cpu.pc = next_pc;
            }

            Opcode::Sb => {
                // sb ra,dd(rb) - store byte
                let base = cpu.get_reg(inst.rb);
                let offset = CpuState::sign_extend_8(imm8);
                let addr = CpuState::mask_24(base.wrapping_add(offset));
                let value = cpu.get_reg(inst.ra) as u8;
                cpu.write_byte(addr, value);
                cpu.pc = next_pc;
            }

            Opcode::Shl => {
                // shl ra,rb - shift left
                let ra_val = cpu.get_reg(inst.ra);
                let shift = cpu.get_reg(inst.rb) & 0x1F;
                let result = CpuState::mask_24(ra_val << shift);
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::Sra => {
                // sra ra,rb - shift right arithmetic
                let ra_val = cpu.get_reg(inst.ra);
                let shift = cpu.get_reg(inst.rb) & 0x1F;
                // Sign extend for arithmetic shift
                let signed_val = if ra_val & 0x800000 != 0 {
                    ra_val | 0xFF000000
                } else {
                    ra_val
                };
                let result = CpuState::mask_24((signed_val as i32 >> shift) as u32);
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::Srl => {
                // srl ra,rb - shift right logical
                let ra_val = cpu.get_reg(inst.ra);
                let shift = cpu.get_reg(inst.rb) & 0x1F;
                let result = ra_val >> shift;
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::Sub => {
                // sub ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                let result = CpuState::mask_24(ra_val.wrapping_sub(rb_val));
                cpu.set_reg(inst.ra, result);
                cpu.pc = next_pc;
            }

            Opcode::SubSp => {
                // sub sp,dddddd
                let sp = cpu.get_reg(4);
                let result = CpuState::mask_24(sp.wrapping_sub(imm24));
                cpu.set_reg(4, result);
                cpu.pc = next_pc;
            }

            Opcode::Sw => {
                // sw ra,dd(rb) - store word
                let base = cpu.get_reg(inst.rb);
                let offset = CpuState::sign_extend_8(imm8);
                let addr = CpuState::mask_24(base.wrapping_add(offset));
                let value = cpu.get_reg(inst.ra);
                cpu.write_word(addr, value);
                cpu.pc = next_pc;
            }

            Opcode::Sxt => {
                // sxt ra,rb - sign extend byte
                let rb_val = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, CpuState::sign_extend_8(rb_val as u8));
                cpu.pc = next_pc;
            }

            Opcode::Xor => {
                // xor ra,rb
                let ra_val = cpu.get_reg(inst.ra);
                let rb_val = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, ra_val ^ rb_val);
                cpu.pc = next_pc;
            }

            Opcode::Zxt => {
                // zxt ra,rb - zero extend byte
                let rb_val = cpu.get_reg(inst.rb);
                cpu.set_reg(inst.ra, rb_val & 0xFF);
                cpu.pc = next_pc;
            }

            Opcode::Invalid => {
                return ExecuteResult::InvalidInstruction(inst_byte);
            }
        }

        cpu.cycles += 1;
        cpu.instructions += 1;
        cpu.uart_tick();

        ExecuteResult::Ok
    }

    /// Run until halted or max cycles reached
    pub fn run(&self, cpu: &mut CpuState, max_cycles: u64) -> ExecuteResult {
        let start_cycles = cpu.cycles;
        while cpu.cycles - start_cycles < max_cycles {
            match self.step(cpu) {
                ExecuteResult::Ok => continue,
                result => return result,
            }
        }
        ExecuteResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::decode_rom::DECODE_ROM;

    /// Helper to find an instruction byte for a given opcode, ra, rb combination
    fn find_instruction_byte(opcode: u8, ra: u8, rb: u8) -> Option<u8> {
        for (byte, &decoded) in DECODE_ROM.iter().enumerate() {
            if decoded == 0xFFF {
                continue;
            }
            let dec_opcode = ((decoded >> 6) & 0x1F) as u8;
            let dec_ra = ((decoded >> 3) & 0x07) as u8;
            let dec_rb = (decoded & 0x07) as u8;
            if dec_opcode == opcode && dec_ra == ra && dec_rb == rb {
                return Some(byte as u8);
            }
        }
        None
    }

    /// Helper to find any instruction byte for a given opcode
    fn find_any_instruction_byte(opcode: u8) -> Option<(u8, u8, u8)> {
        for (byte, &decoded) in DECODE_ROM.iter().enumerate() {
            if decoded == 0xFFF {
                continue;
            }
            let dec_opcode = ((decoded >> 6) & 0x1F) as u8;
            let dec_ra = ((decoded >> 3) & 0x07) as u8;
            let dec_rb = (decoded & 0x07) as u8;
            if dec_opcode == opcode {
                return Some((byte as u8, dec_ra, dec_rb));
            }
        }
        None
    }

    /// Helper to find an instruction byte for a given opcode where ra != rb
    fn find_instruction_byte_different_regs(opcode: u8) -> Option<(u8, u8, u8)> {
        for (byte, &decoded) in DECODE_ROM.iter().enumerate() {
            if decoded == 0xFFF {
                continue;
            }
            let dec_opcode = ((decoded >> 6) & 0x1F) as u8;
            let dec_ra = ((decoded >> 3) & 0x07) as u8;
            let dec_rb = (decoded & 0x07) as u8;
            if dec_opcode == opcode && dec_ra != dec_rb {
                return Some((byte as u8, dec_ra, dec_rb));
            }
        }
        None
    }

    // ========== AddReg (opcode 0x00) ==========

    #[test]
    fn test_add_reg_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,r1 (0x01)
        let inst_byte = find_instruction_byte(0x00, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 10);
        cpu.set_reg(1, 20);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 30);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_add_reg_overflow() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,r1 - test 24-bit overflow
        let inst_byte = find_instruction_byte(0x00, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFFFFFF);
        cpu.set_reg(1, 1);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0); // Wraps to 0
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_add_reg_same_register() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,r0 (doubles the value)
        // Note: byte 0 at address 0 triggers halt, so we put it at address 10
        let inst_byte = find_instruction_byte(0x00, 0, 0).unwrap();
        cpu.pc = 10;
        cpu.write_byte(10, inst_byte);
        cpu.set_reg(0, 100);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 200);
    }

    // ========== AddImm (opcode 0x01) ==========

    #[test]
    fn test_add_immediate() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,5 (0x09 is add r0,imm)
        cpu.write_byte(0, 0x09);
        cpu.write_byte(1, 0x05);
        cpu.set_reg(0, 10);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 15);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_add_immediate_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,-1 (0xFF sign-extended)
        cpu.write_byte(0, 0x09);
        cpu.write_byte(1, 0xFF); // -1 as signed byte
        cpu.set_reg(0, 10);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 9);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_add_immediate_large_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // add r0,-128 (0x80 sign-extended)
        cpu.write_byte(0, 0x09);
        cpu.write_byte(1, 0x80); // -128 as signed byte
        cpu.set_reg(0, 200);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 72); // 200 - 128 = 72
    }

    // ========== And (opcode 0x02) ==========

    #[test]
    fn test_and_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // and ra,rb - use any available encoding
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x02).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 0xFF00FF);
        cpu.set_reg(rb, 0x0FF0F0);

        executor.step(&mut cpu);

        // 0xFF00FF & 0x0FF0F0 = 0x0F00F0
        assert_eq!(cpu.get_reg(ra), 0x0F00F0);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_and_all_ones() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x02).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 0xFFFFFF);
        cpu.set_reg(rb, 0xFFFFFF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xFFFFFF);
    }

    #[test]
    fn test_and_zero() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x02).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 0xFFFFFF);
        cpu.set_reg(rb, 0x000000);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0);
    }

    // ========== Bra (opcode 0x03) ==========

    #[test]
    fn test_bra_forward() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // bra +10
        cpu.write_byte(0, 0x13); // bra opcode
        cpu.write_byte(1, 10);

        executor.step(&mut cpu);

        // PC starts at 0, branch_base = 0+2+2 = 4, then offset +10 = 14
        assert_eq!(cpu.pc, 14);
    }

    #[test]
    fn test_bra_backward() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        cpu.pc = 100;
        // bra -10 (0xF6 = -10 in two's complement)
        cpu.write_byte(100, 0x13);
        cpu.write_byte(101, 0xF6); // -10

        executor.step(&mut cpu);

        // PC was 100, branch_base = 100+2+2 = 104, then offset -10 = 94
        assert_eq!(cpu.pc, 94);
    }

    // ========== Brf (opcode 0x04) ==========

    #[test]
    fn test_brf_taken() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // brf when c is false - branch should be taken
        cpu.c = false;
        cpu.write_byte(0, 0x14); // brf opcode
        cpu.write_byte(1, 20);

        executor.step(&mut cpu);

        // PC = 0, branch_base = 0+2+2 = 4, offset +20 = 24
        assert_eq!(cpu.pc, 24);
    }

    #[test]
    fn test_brf_not_taken() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // brf when c is true - branch should NOT be taken
        cpu.c = true;
        cpu.write_byte(0, 0x14);
        cpu.write_byte(1, 20);

        executor.step(&mut cpu);

        assert_eq!(cpu.pc, 2);
    }

    // ========== Brt (opcode 0x05) ==========

    #[test]
    fn test_brt_taken() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // brt when c is true - branch should be taken
        cpu.c = true;
        cpu.write_byte(0, 0x15); // brt opcode
        cpu.write_byte(1, 30);

        executor.step(&mut cpu);

        assert_eq!(cpu.pc, 34); // 0 + 2 + 2 + 30
    }

    #[test]
    fn test_brt_not_taken() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // brt when c is false - branch should NOT be taken
        cpu.c = false;
        cpu.write_byte(0, 0x15);
        cpu.write_byte(1, 30);

        executor.step(&mut cpu);

        assert_eq!(cpu.pc, 2);
    }

    // ========== Ceq (opcode 0x06) ==========

    #[test]
    fn test_ceq_equal() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // ceq r0,r1
        let inst_byte = find_instruction_byte(0x06, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 42);
        cpu.set_reg(1, 42);

        executor.step(&mut cpu);

        assert!(cpu.c);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_ceq_not_equal() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x06, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 42);
        cpu.set_reg(1, 43);

        executor.step(&mut cpu);

        assert!(!cpu.c);
    }

    // ========== Cls (opcode 0x07) - Compare Less Signed ==========

    #[test]
    fn test_cls_less_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // cls r0,r1 - compare if r0 < r1 (signed)
        let inst_byte = find_instruction_byte(0x07, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 10);
        cpu.set_reg(1, 20);

        executor.step(&mut cpu);

        assert!(cpu.c); // 10 < 20 is true
    }

    #[test]
    fn test_cls_greater_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x07, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 20);
        cpu.set_reg(1, 10);

        executor.step(&mut cpu);

        assert!(!cpu.c); // 20 < 10 is false
    }

    #[test]
    fn test_cls_negative_vs_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x07, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFFFFFF); // -1 in 24-bit signed
        cpu.set_reg(1, 1);

        executor.step(&mut cpu);

        assert!(cpu.c); // -1 < 1 is true (signed comparison)
    }

    #[test]
    fn test_cls_both_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x07, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFFFFFE); // -2 in 24-bit signed
        cpu.set_reg(1, 0xFFFFFF); // -1 in 24-bit signed

        executor.step(&mut cpu);

        assert!(cpu.c); // -2 < -1 is true
    }

    // ========== Clu (opcode 0x08) - Compare Less Unsigned ==========

    #[test]
    fn test_clu_less() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x08, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 10);
        cpu.set_reg(1, 20);

        executor.step(&mut cpu);

        assert!(cpu.c);
    }

    #[test]
    fn test_clu_greater() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x08, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 20);
        cpu.set_reg(1, 10);

        executor.step(&mut cpu);

        assert!(!cpu.c);
    }

    #[test]
    fn test_clu_high_value_vs_low() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x08, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFFFFFF); // Large unsigned value
        cpu.set_reg(1, 1);

        executor.step(&mut cpu);

        assert!(!cpu.c); // 0xFFFFFF > 1 unsigned
    }

    // ========== Jal (opcode 0x09) - Jump and Link ==========

    #[test]
    fn test_jal() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // jal ra,(rb) - jump to rb, store return address in ra
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x09).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(rb, 0x1000); // Target address

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 1); // Return address (next_pc)
        assert_eq!(cpu.pc, 0x1000);
    }

    #[test]
    fn test_jal_link_register() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x09).unwrap();
        cpu.pc = 0x500;
        cpu.write_byte(0x500, inst_byte);
        cpu.set_reg(rb, 0x2000);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x501); // Return address
        assert_eq!(cpu.pc, 0x2000);
    }

    // ========== Jmp (opcode 0x0A) - Jump ==========

    #[test]
    fn test_jmp() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // jmp (ra)
        let (inst_byte, ra, _) = find_any_instruction_byte(0x0A).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 0x3000);

        executor.step(&mut cpu);

        assert_eq!(cpu.pc, 0x3000);
    }

    // ========== La (opcode 0x0B) - Load Address ==========

    #[test]
    fn test_la_load_address() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // la r0,0x123456
        let (inst_byte, ra, _) = find_any_instruction_byte(0x0B).unwrap();
        // Make sure it's not r7 (which would be a jump)
        if ra == 7 {
            // Skip this test if we can only find r7 variant
            return;
        }
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x56);
        cpu.write_byte(2, 0x34);
        cpu.write_byte(3, 0x12);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x123456);
        assert_eq!(cpu.pc, 4);
    }

    #[test]
    fn test_la_absolute_jump() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // la r7,addr is actually jmp addr (absolute jump)
        // Find the la r7 encoding
        let inst_byte = find_instruction_byte(0x0B, 7, 0);
        if inst_byte.is_none() {
            // Try to find any la r7 encoding
            return;
        }
        let inst_byte = inst_byte.unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00);
        cpu.write_byte(2, 0x10);
        cpu.write_byte(3, 0x00);

        executor.step(&mut cpu);

        assert_eq!(cpu.pc, 0x001000);
    }

    // ========== Lb (opcode 0x0C) - Load Byte Signed ==========

    #[test]
    fn test_lb_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // lb r0,offset(r1)
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x0C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x10); // offset = 16
        cpu.set_reg(rb, 0x100); // base address
        cpu.write_byte(0x110, 0x42); // value at address

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x42);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_lb_negative_sign_extend() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x0C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00); // offset = 0
        cpu.set_reg(rb, 0x100);
        cpu.write_byte(0x100, 0x80); // -128 as signed byte

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xFFFF80); // Sign extended
    }

    #[test]
    fn test_lb_negative_offset() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x0C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0xF0); // offset = -16
        cpu.set_reg(rb, 0x200);
        cpu.write_byte(0x1F0, 0x55); // value at 0x200 - 16

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x55);
    }

    // ========== Lbu (opcode 0x0D) - Load Byte Unsigned ==========

    #[test]
    fn test_lbu() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x0D).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00);
        cpu.set_reg(rb, 0x100);
        cpu.write_byte(0x100, 0xFF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xFF); // NOT sign extended
    }

    #[test]
    fn test_lbu_high_bit() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x0D).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00);
        cpu.set_reg(rb, 0x100);
        cpu.write_byte(0x100, 0x80);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x80); // Zero extended, not sign extended
    }

    // ========== Lc (opcode 0x0E) - Load Constant Signed ==========

    #[test]
    fn test_lc() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // lc r0,42 (0x44 = lc r0)
        cpu.write_byte(0, 0x44);
        cpu.write_byte(1, 42);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 42);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_lc_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        cpu.write_byte(0, 0x44);
        cpu.write_byte(1, 0xFF); // -1

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFFFF); // Sign extended
    }

    #[test]
    fn test_lc_negative_128() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        cpu.write_byte(0, 0x44);
        cpu.write_byte(1, 0x80); // -128

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFF80);
    }

    // ========== Lcu (opcode 0x0F) - Load Constant Unsigned ==========

    #[test]
    fn test_lcu() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, _) = find_any_instruction_byte(0x0F).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0xFF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xFF); // Zero extended
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_lcu_high_value() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, _) = find_any_instruction_byte(0x0F).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x80);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x80); // NOT sign extended
    }

    // ========== Lw (opcode 0x10) - Load Word ==========

    #[test]
    fn test_lw() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x10).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00); // offset
        cpu.set_reg(rb, 0x100);
        cpu.write_word(0x100, 0x123456);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0x123456);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_lw_with_offset() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x10).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x10); // offset = 16
        cpu.set_reg(rb, 0x100);
        cpu.write_word(0x110, 0xABCDEF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xABCDEF);
    }

    // ========== Mov (opcode 0x11) ==========

    #[test]
    fn test_mov_register() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // mov r0,r1
        let inst_byte = find_instruction_byte(0x11, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x123456);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x123456);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_mov_from_c_true() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // mov ra,c (rb=5 means move from condition flag)
        let inst_byte = find_instruction_byte(0x11, 0, 5);
        if inst_byte.is_none() {
            return; // encoding not available
        }
        cpu.write_byte(0, inst_byte.unwrap());
        cpu.c = true;

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 1);
    }

    #[test]
    fn test_mov_from_c_false() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x11, 0, 5);
        if inst_byte.is_none() {
            return;
        }
        cpu.write_byte(0, inst_byte.unwrap());
        cpu.c = false;

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0);
    }

    // ========== Mul (opcode 0x12) ==========

    #[test]
    fn test_mul_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x12, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 10);
        cpu.set_reg(1, 20);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 200);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_mul_overflow() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x12, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x1000);
        cpu.set_reg(1, 0x1000);

        executor.step(&mut cpu);

        // 0x1000 * 0x1000 = 0x1000000, masked to 24 bits = 0
        assert_eq!(cpu.get_reg(0), 0);
    }

    #[test]
    fn test_mul_by_zero() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x12, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 12345);
        cpu.set_reg(1, 0);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0);
    }

    // ========== Or (opcode 0x13) ==========

    #[test]
    fn test_or_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x13, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x0F0F0F);
        cpu.set_reg(1, 0xF0F0F0);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFFFF);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_or_with_zero() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x13, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xABCDEF);
        cpu.set_reg(1, 0);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xABCDEF);
    }

    // ========== Pop (opcode 0x14) ==========

    #[test]
    fn test_pop() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // pop r0
        let inst_byte = find_instruction_byte(0x14, 0, 4).unwrap();
        let sp = 0x1000;
        cpu.set_reg(4, sp); // sp
        cpu.write_word(sp, 0x123456);
        cpu.write_byte(0, inst_byte);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x123456);
        assert_eq!(cpu.get_reg(4), sp + 3); // sp incremented by 3 (word size)
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_pop_multiple() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let sp = 0x1000;
        cpu.set_reg(4, sp);
        cpu.write_word(sp, 0x111111);
        cpu.write_word(sp + 3, 0x222222);

        // First pop
        let inst_byte = find_instruction_byte(0x14, 0, 4).unwrap();
        cpu.write_byte(0, inst_byte);
        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x111111);
        assert_eq!(cpu.get_reg(4), sp + 3);

        // Second pop (different register)
        let inst_byte2 = find_instruction_byte(0x14, 1, 4).unwrap();
        cpu.write_byte(1, inst_byte2);
        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(1), 0x222222);
        assert_eq!(cpu.get_reg(4), sp + 6);
    }

    // ========== Push (opcode 0x15) ==========

    #[test]
    fn test_push() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x15, 0, 4).unwrap();
        let sp = 0x1000;
        cpu.set_reg(4, sp);
        cpu.set_reg(0, 0xABCDEF);
        cpu.write_byte(0, inst_byte);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(4), sp - 3); // sp decremented by 3
        assert_eq!(cpu.read_word(sp - 3), 0xABCDEF);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_push_pop_roundtrip() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let initial_sp = 0x1000;
        cpu.set_reg(4, initial_sp);
        cpu.set_reg(0, 0x123456);

        // Push
        let push_byte = find_instruction_byte(0x15, 0, 4).unwrap();
        cpu.write_byte(0, push_byte);
        executor.step(&mut cpu);

        // Pop to different register
        let pop_byte = find_instruction_byte(0x14, 1, 4).unwrap();
        cpu.write_byte(1, pop_byte);
        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(1), 0x123456);
        assert_eq!(cpu.get_reg(4), initial_sp);
    }

    // ========== Sb (opcode 0x16) - Store Byte ==========

    #[test]
    fn test_sb() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x16).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x10); // offset
        cpu.set_reg(ra, 0x1234AB);
        cpu.set_reg(rb, 0x200);

        executor.step(&mut cpu);

        assert_eq!(cpu.read_byte(0x210), 0xAB); // Only low byte stored
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_sb_negative_offset() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_any_instruction_byte(0x16).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0xF0); // offset = -16
        cpu.set_reg(ra, 0x55);
        cpu.set_reg(rb, 0x200);

        executor.step(&mut cpu);

        assert_eq!(cpu.read_byte(0x1F0), 0x55);
    }

    // ========== Shl (opcode 0x17) - Shift Left ==========

    #[test]
    fn test_shl_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x17, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x01);
        cpu.set_reg(1, 4);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x10);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_shl_overflow() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x17, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFF);
        cpu.set_reg(1, 20);

        executor.step(&mut cpu);

        // 0xFF << 20 = 0xFF00000, masked to 24 bits = 0xF00000
        assert_eq!(cpu.get_reg(0), 0xF00000);
    }

    #[test]
    fn test_shl_by_zero() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x17, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x123);
        cpu.set_reg(1, 0);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x123);
    }

    // ========== Sra (opcode 0x18) - Shift Right Arithmetic ==========

    #[test]
    fn test_sra_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x18, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x100);
        cpu.set_reg(1, 4);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x10);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_sra_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x18, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x800000); // Negative in 24-bit signed
        cpu.set_reg(1, 4);

        executor.step(&mut cpu);

        // Should preserve sign bit
        assert_eq!(cpu.get_reg(0), 0xF80000);
    }

    #[test]
    fn test_sra_all_ones() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x18, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFFFFFF); // -1
        cpu.set_reg(1, 8);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFFFF); // Still -1
    }

    // ========== Srl (opcode 0x19) - Shift Right Logical ==========

    #[test]
    fn test_srl_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x19, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x100);
        cpu.set_reg(1, 4);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x10);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_srl_high_bit() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x19, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0x800000);
        cpu.set_reg(1, 4);

        executor.step(&mut cpu);

        // Should NOT preserve sign bit (logical shift)
        assert_eq!(cpu.get_reg(0), 0x080000);
    }

    #[test]
    fn test_srl_vs_sra() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // Test that srl and sra differ for negative numbers
        let (srl_byte, srl_ra, srl_rb) = find_any_instruction_byte(0x19).unwrap();
        let (sra_byte, sra_ra, sra_rb) = find_any_instruction_byte(0x18).unwrap();

        // SRL
        cpu.write_byte(0, srl_byte);
        cpu.set_reg(srl_ra, 0xF00000);
        cpu.set_reg(srl_rb, 4);
        executor.step(&mut cpu);
        let srl_result = cpu.get_reg(srl_ra);

        // SRA
        cpu.write_byte(1, sra_byte);
        cpu.set_reg(sra_ra, 0xF00000);
        cpu.set_reg(sra_rb, 4);
        executor.step(&mut cpu);
        let sra_result = cpu.get_reg(sra_ra);

        assert_eq!(srl_result, 0x0F0000); // Logical: zeros shifted in
        assert_eq!(sra_result, 0xFF0000); // Arithmetic: sign bit preserved
    }

    // ========== Sub (opcode 0x1A) ==========

    #[test]
    fn test_sub_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1A, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 100);
        cpu.set_reg(1, 30);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 70);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_sub_underflow() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1A, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0);
        cpu.set_reg(1, 1);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFFFF); // Wraps to -1
    }

    #[test]
    fn test_sub_same_register() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // Find a sub encoding where ra == rb (if it exists)
        // If not, we'll just test subtraction with same value in both regs
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x1A).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 12345);
        cpu.set_reg(rb, 12345);

        executor.step(&mut cpu);

        // ra = ra - rb = 12345 - 12345 = 0
        assert_eq!(cpu.get_reg(ra), 0);
    }

    // ========== SubSp (opcode 0x1B) ==========

    #[test]
    fn test_sub_sp() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // sub sp,dddddd
        let (inst_byte, _, _) = find_any_instruction_byte(0x1B).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x10); // 16
        cpu.write_byte(2, 0x00);
        cpu.write_byte(3, 0x00);
        cpu.set_reg(4, 0x1000); // sp

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(4), 0x1000 - 16);
        assert_eq!(cpu.pc, 4);
    }

    #[test]
    fn test_sub_sp_large() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, _, _) = find_any_instruction_byte(0x1B).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00);
        cpu.write_byte(2, 0x01);
        cpu.write_byte(3, 0x00); // 256
        cpu.set_reg(4, 0x2000);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(4), 0x2000 - 256);
    }

    // ========== Sw (opcode 0x1C) - Store Word ==========

    #[test]
    fn test_sw() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // sw ra,dd(rb) - store ra at address [rb + offset]
        // Need an encoding where ra != rb to properly test
        let (inst_byte, ra, rb) = find_instruction_byte_different_regs(0x1C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00); // offset = 0
        cpu.set_reg(ra, 0x123456);
        cpu.set_reg(rb, 0x200);

        executor.step(&mut cpu);

        assert_eq!(cpu.read_word(0x200), 0x123456);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_sw_with_offset() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_instruction_byte_different_regs(0x1C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x10); // offset = 16
        cpu.set_reg(ra, 0xABCDEF);
        cpu.set_reg(rb, 0x200);

        executor.step(&mut cpu);

        assert_eq!(cpu.read_word(0x210), 0xABCDEF);
    }

    #[test]
    fn test_sw_negative_offset() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let (inst_byte, ra, rb) = find_instruction_byte_different_regs(0x1C).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0xF0); // offset = -16
        cpu.set_reg(ra, 0xABCDEF);
        cpu.set_reg(rb, 0x200);

        executor.step(&mut cpu);

        assert_eq!(cpu.read_word(0x1F0), 0xABCDEF);
    }

    #[test]
    fn test_sw_lw_roundtrip() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // Store a value using SW (need different ra/rb)
        let (sw_byte, sw_ra, sw_rb) = find_instruction_byte_different_regs(0x1C).unwrap();
        cpu.write_byte(0, sw_byte);
        cpu.write_byte(1, 0x00); // offset = 0
        let test_value = 0x987654;
        let test_addr = 0x400;
        cpu.set_reg(sw_ra, test_value);
        cpu.set_reg(sw_rb, test_addr);
        executor.step(&mut cpu);

        // Verify it's stored
        assert_eq!(cpu.read_word(test_addr), test_value);

        // Load it back using LW (need different ra/rb)
        let (lw_byte, lw_ra, lw_rb) = find_instruction_byte_different_regs(0x10).unwrap();
        cpu.write_byte(2, lw_byte);
        cpu.write_byte(3, 0x00); // offset = 0
        cpu.set_reg(lw_rb, test_addr);
        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(lw_ra), test_value);
    }

    // ========== Sxt (opcode 0x1D) - Sign Extend Byte ==========

    #[test]
    fn test_sxt_positive() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1D, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x7F);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x7F);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_sxt_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1D, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x80);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFF80);
    }

    #[test]
    fn test_sxt_full_negative() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1D, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0xFF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xFFFFFF);
    }

    #[test]
    fn test_sxt_high_bits_ignored() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1D, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x123456); // High bits should be ignored

        executor.step(&mut cpu);

        // Only looks at low byte 0x56 (positive)
        assert_eq!(cpu.get_reg(0), 0x56);
    }

    // ========== Xor (opcode 0x1E) ==========

    #[test]
    fn test_xor_basic() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1E, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xFF00FF);
        cpu.set_reg(1, 0x0F0F0F);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xF00FF0);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_xor_same_value() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // XOR with same value in both registers gives 0
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x1E).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(ra, 0x123456);
        cpu.set_reg(rb, 0x123456);

        executor.step(&mut cpu);

        // ra = ra ^ rb = 0x123456 ^ 0x123456 = 0
        assert_eq!(cpu.get_reg(ra), 0);
    }

    #[test]
    fn test_xor_with_zero() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1E, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(0, 0xABCDEF);
        cpu.set_reg(1, 0);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xABCDEF);
    }

    // ========== Zxt (opcode 0x1F) - Zero Extend Byte ==========

    #[test]
    fn test_zxt() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1F, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x123456);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x56); // Only low byte
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_zxt_high_bit() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x1F, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.set_reg(1, 0x80);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x80); // NOT sign extended
    }

    #[test]
    fn test_zxt_vs_sxt() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // ZXT
        let (zxt_byte, zxt_ra, zxt_rb) = find_any_instruction_byte(0x1F).unwrap();
        cpu.write_byte(0, zxt_byte);
        cpu.set_reg(zxt_rb, 0x80);
        executor.step(&mut cpu);
        let zxt_result = cpu.get_reg(zxt_ra);

        // SXT
        let (sxt_byte, sxt_ra, sxt_rb) = find_any_instruction_byte(0x1D).unwrap();
        cpu.write_byte(1, sxt_byte);
        cpu.set_reg(sxt_rb, 0x80);
        executor.step(&mut cpu);
        let sxt_result = cpu.get_reg(sxt_ra);

        assert_eq!(zxt_result, 0x80);
        assert_eq!(sxt_result, 0xFFFF80);
    }

    // ========== Edge Cases and Integration Tests ==========

    #[test]
    fn test_halt_condition() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // 0x00 at address 0 is halt
        cpu.write_byte(0, 0x00);
        cpu.pc = 0;

        let result = executor.step(&mut cpu);

        assert_eq!(result, ExecuteResult::Halted);
        assert!(cpu.halted);
    }

    #[test]
    fn test_invalid_instruction() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        cpu.pc = 10; // Not at address 0, so 0x00 isn't halt
        cpu.write_byte(10, 0xFF); // Invalid instruction

        let result = executor.step(&mut cpu);

        assert!(matches!(result, ExecuteResult::InvalidInstruction(_)));
    }

    #[test]
    fn test_instruction_count() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        let inst_byte = find_instruction_byte(0x00, 0, 1).unwrap();
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, inst_byte);
        cpu.write_byte(2, inst_byte);

        assert_eq!(cpu.instructions, 0);

        executor.step(&mut cpu);
        assert_eq!(cpu.instructions, 1);

        executor.step(&mut cpu);
        assert_eq!(cpu.instructions, 2);
    }

    #[test]
    fn test_run_until_halt() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // Simple program that jumps to address 0 to halt
        // Use la r7,0 which acts as jmp 0 (absolute jump)
        // Find la r7 encoding
        let inst_byte = find_instruction_byte(0x0B, 7, 0);
        if inst_byte.is_none() {
            // If la r7 isn't available, skip this test
            return;
        }

        // Set up: la r7,0 at address 10 (jumps to 0, which triggers halt)
        cpu.write_byte(10, inst_byte.unwrap());
        cpu.write_byte(11, 0x00);
        cpu.write_byte(12, 0x00);
        cpu.write_byte(13, 0x00);
        cpu.write_byte(0, 0x00); // halt at address 0

        cpu.pc = 10;

        let result = executor.run(&mut cpu, 100);

        assert_eq!(result, ExecuteResult::Halted);
    }

    #[test]
    fn test_branch_sequence() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // ceq r0,r1 -> brt +10
        let ceq_byte = find_instruction_byte(0x06, 0, 1).unwrap();
        cpu.write_byte(0, ceq_byte);
        cpu.write_byte(1, 0x15); // brt
        cpu.write_byte(2, 10); // offset

        cpu.set_reg(0, 42);
        cpu.set_reg(1, 42);

        executor.step(&mut cpu); // ceq
        assert!(cpu.c);
        assert_eq!(cpu.pc, 1);

        executor.step(&mut cpu); // brt
        assert_eq!(cpu.pc, 15); // 1 + 2 + 2 + 10 = 15
    }

    #[test]
    fn test_24bit_address_masking() {
        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // la r0,0xFFFFFF
        let (inst_byte, ra, _) = find_any_instruction_byte(0x0B).unwrap();
        if ra == 7 {
            return;
        }
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0xFF);
        cpu.write_byte(2, 0xFF);
        cpu.write_byte(3, 0xFF);

        executor.step(&mut cpu);

        assert_eq!(cpu.get_reg(ra), 0xFFFFFF);
    }

    // ========== LED I/O Test ==========

    #[test]
    fn test_sb_to_led() {
        use crate::cpu::state::IO_LEDSWDAT;

        let mut cpu = CpuState::new();
        let executor = Executor::new();

        // sb r1, 0(r0) where r0 = LED address, r1 = value
        let (inst_byte, ra, rb) = find_any_instruction_byte(0x16).unwrap(); // Sb opcode
        cpu.write_byte(0, inst_byte);
        cpu.write_byte(1, 0x00); // offset = 0
        cpu.set_reg(ra, 0x55); // Value to write to LEDs
        cpu.set_reg(rb, IO_LEDSWDAT); // LED I/O address

        executor.step(&mut cpu);

        assert_eq!(cpu.io.leds, 0x55, "LED value should be 0x55");
    }

    // ========== Sieve Integration Test ==========

    #[test]
    fn test_sieve_lgo_produces_correct_output() {
        let lgo = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/docs/research/asld24/sieve.lgo"),
        )
        .expect("sieve.lgo must exist");

        let mut cpu = CpuState::new();
        crate::loader::load_lgo(&lgo, &mut cpu).unwrap();

        // Entry point is _main at 0x93 (no G line in sieve.lgo)
        cpu.pc = 0x93;

        let executor = Executor::new();
        executor.run(&mut cpu, 500_000_000);

        assert_eq!(
            cpu.io.uart_output,
            "1000 iterations\n1899 primes.\n",
            "Sieve should output correct prime count"
        );
    }
}
