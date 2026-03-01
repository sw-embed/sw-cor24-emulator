//! Complete Rust→COR24 Pipeline
//!
//! This module provides an end-to-end pipeline:
//! 1. Compile Rust to WASM (using rustc)
//! 2. Translate WASM to COR24 assembly (wasm2cor24)
//! 3. Assemble COR24 assembly to machine code
//! 4. Run on emulator with LED output

use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Memory-mapped I/O address for LEDs
const IO_LEDSWDAT: u32 = 0xFF0000;

/// Minimal COR24 CPU state
pub struct Cpu {
    pc: u32,
    regs: [u32; 8],
    c: bool,
    mem: Vec<u8>,
    halted: bool,
    pub leds: u8,
    prev_leds: u8,
    pub led_changes: Vec<u8>,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            pc: 0,
            regs: [0; 8],
            c: false,
            mem: vec![0; 65536],
            halted: false,
            leds: 0,
            prev_leds: 0xFF, // Start different so first write shows
            led_changes: Vec::new(),
        };
        // Initialize stack pointer to top of RAM (below I/O region)
        cpu.regs[4] = 0xFE00;
        cpu
    }

    fn mask24(v: u32) -> u32 {
        v & 0xFFFFFF
    }

    fn sign_ext8(v: u8) -> u32 {
        if v & 0x80 != 0 {
            0xFFFF00 | (v as u32)
        } else {
            v as u32
        }
    }

    fn sign_ext24(v: u32) -> i32 {
        if v & 0x800000 != 0 {
            (v | 0xFF000000) as i32
        } else {
            v as i32
        }
    }

    fn read_byte(&self, addr: u32) -> u8 {
        let addr = addr & 0xFFFFFF;
        if (addr & 0xFF0000) == 0xFF0000 {
            0 // I/O read returns 0 (switches)
        } else {
            let len = self.mem.len();
            self.mem[(addr as usize) % len]
        }
    }

    fn write_byte(&mut self, addr: u32, val: u8) {
        let addr = addr & 0xFFFFFF;
        if addr == IO_LEDSWDAT {
            self.leds = val;
            if self.leds != self.prev_leds {
                self.led_changes.push(self.leds);
                self.prev_leds = self.leds;
            }
        } else if (addr & 0xFF0000) != 0xFF0000 {
            let len = self.mem.len();
            self.mem[(addr as usize) % len] = val;
        }
    }

    fn get_reg(&self, r: u8) -> u32 {
        if r == 5 {
            0 // z register always 0
        } else {
            self.regs[(r & 7) as usize] & 0xFFFFFF
        }
    }

    fn set_reg(&mut self, r: u8, v: u32) {
        if r != 5 {
            // Can't write to z register
            self.regs[(r & 7) as usize] = v & 0xFFFFFF;
        }
    }

    pub fn load_program(&mut self, data: &[u8]) {
        for (i, &b) in data.iter().enumerate() {
            if i < self.mem.len() {
                self.mem[i] = b;
            }
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Execute one instruction
    pub fn step(&mut self) -> bool {
        if self.halted {
            return false;
        }

        let b0 = self.read_byte(self.pc);

        // Decode based on first byte
        match b0 {
            // halt (encoded as la ir,0)
            0xC7 => {
                let b1 = self.read_byte(self.pc + 1);
                let b2 = self.read_byte(self.pc + 2);
                let b3 = self.read_byte(self.pc + 3);
                let addr = (b1 as u32) | ((b2 as u32) << 8) | ((b3 as u32) << 16);
                if addr == 0 {
                    self.halted = true;
                    return false;
                }
                // jmp absolute
                self.pc = addr;
            }

            // la ra,imm24 (0x29-0x2F for r0-r6)
            0x29..=0x2F => {
                let ra = b0 - 0x29;
                let b1 = self.read_byte(self.pc + 1);
                let b2 = self.read_byte(self.pc + 2);
                let b3 = self.read_byte(self.pc + 3);
                let imm24 = (b1 as u32) | ((b2 as u32) << 8) | ((b3 as u32) << 16);
                self.set_reg(ra, imm24);
                self.pc = Self::mask24(self.pc + 4);
            }

            // lc ra,imm8 (0x44-0x4B)
            0x44..=0x47 => {
                let ra = b0 - 0x44;
                let imm = self.read_byte(self.pc + 1);
                self.set_reg(ra, Self::sign_ext8(imm));
                self.pc = Self::mask24(self.pc + 2);
            }

            // lcu ra,imm8 (0x48-0x4B)
            0x48..=0x4B => {
                let ra = b0 - 0x48;
                let imm = self.read_byte(self.pc + 1);
                self.set_reg(ra, imm as u32);
                self.pc = Self::mask24(self.pc + 2);
            }

            // add ra,imm8 (0x09, 0x11, 0x19, 0x21 for r0-r3)
            0x09 => {
                let imm = self.read_byte(self.pc + 1);
                let v = Self::mask24(self.get_reg(0).wrapping_add(Self::sign_ext8(imm)));
                self.set_reg(0, v);
                self.pc = Self::mask24(self.pc + 2);
            }
            0x11 => {
                let imm = self.read_byte(self.pc + 1);
                let v = Self::mask24(self.get_reg(1).wrapping_add(Self::sign_ext8(imm)));
                self.set_reg(1, v);
                self.pc = Self::mask24(self.pc + 2);
            }
            0x19 => {
                let imm = self.read_byte(self.pc + 1);
                let v = Self::mask24(self.get_reg(2).wrapping_add(Self::sign_ext8(imm)));
                self.set_reg(2, v);
                self.pc = Self::mask24(self.pc + 2);
            }

            // add ra,rb (register-register)
            // Encoding: varies by register pair
            0x00 => {
                // add r0,r0
                let v = Self::mask24(self.get_reg(0).wrapping_add(self.get_reg(0)));
                self.set_reg(0, v);
                self.pc = Self::mask24(self.pc + 1);
            }
            0x01 => {
                // add r0,r1
                let v = Self::mask24(self.get_reg(0).wrapping_add(self.get_reg(1)));
                self.set_reg(0, v);
                self.pc = Self::mask24(self.pc + 1);
            }
            0x02 => {
                // add r0,r2
                let v = Self::mask24(self.get_reg(0).wrapping_add(self.get_reg(2)));
                self.set_reg(0, v);
                self.pc = Self::mask24(self.pc + 1);
            }

            // mov ra,rb (0x30-0x3F pattern)
            0x30 => { self.set_reg(0, self.get_reg(0)); self.pc += 1; }
            0x31 => { self.set_reg(0, self.get_reg(1)); self.pc += 1; }
            0x32 => { self.set_reg(0, self.get_reg(2)); self.pc += 1; }
            0x38 => { self.set_reg(1, self.get_reg(0)); self.pc += 1; }
            0x39 => { self.set_reg(1, self.get_reg(1)); self.pc += 1; }
            0x3A => { self.set_reg(1, self.get_reg(2)); self.pc += 1; }
            0x40 => { self.set_reg(2, self.get_reg(0)); self.pc += 1; }
            0x41 => { self.set_reg(2, self.get_reg(1)); self.pc += 1; }
            0x42 => { self.set_reg(2, self.get_reg(2)); self.pc += 1; }

            // mov ra,c (move condition to register)
            0x34 => { self.set_reg(0, if self.c { 1 } else { 0 }); self.pc += 1; }
            0x3C => { self.set_reg(1, if self.c { 1 } else { 0 }); self.pc += 1; }
            0x43 => { self.set_reg(2, if self.c { 1 } else { 0 }); self.pc += 1; }

            // sb ra,imm(rb) - store byte
            // Encoding: 0x80 + ra*8 + rb
            0x80..=0x89 => {
                let idx = b0 - 0x80;
                let ra = idx / 8;
                let rb = idx % 8;
                let imm = self.read_byte(self.pc + 1);
                let addr = Self::mask24(self.get_reg(rb).wrapping_add(Self::sign_ext8(imm)));
                self.write_byte(addr, self.get_reg(ra) as u8);
                self.pc = Self::mask24(self.pc + 2);
            }

            // sw ra,imm(fp) - store word to stack frame
            // Encoding: 0x8A + ra (ra = 0..2)
            0x8A..=0x8C => {
                let ra = b0 - 0x8A;
                let imm = self.read_byte(self.pc + 1);
                let addr = Self::mask24(self.get_reg(3).wrapping_add(Self::sign_ext8(imm)));
                let v = self.get_reg(ra);
                self.write_byte(addr, (v & 0xFF) as u8);
                self.write_byte(addr + 1, ((v >> 8) & 0xFF) as u8);
                self.write_byte(addr + 2, ((v >> 16) & 0xFF) as u8);
                self.pc = Self::mask24(self.pc + 2);
            }

            // lw ra,imm(fp) - load word from stack frame
            // Encoding: 0x92 + ra (ra = 0..2)
            0x92..=0x94 => {
                let ra = b0 - 0x92;
                let imm = self.read_byte(self.pc + 1);
                let addr = Self::mask24(self.get_reg(3).wrapping_add(Self::sign_ext8(imm)));
                let b0 = self.read_byte(addr) as u32;
                let b1 = self.read_byte(addr + 1) as u32;
                let b2 = self.read_byte(addr + 2) as u32;
                self.set_reg(ra, b0 | (b1 << 8) | (b2 << 16));
                self.pc = Self::mask24(self.pc + 2);
            }

            // and ra,rb
            0x03 => {
                // and r0,r0 (no-op)
                self.pc += 1;
            }
            0x04 => {
                // and r0,r1
                self.set_reg(0, self.get_reg(0) & self.get_reg(1));
                self.pc += 1;
            }
            0x05 => {
                // and r0,r2
                self.set_reg(0, self.get_reg(0) & self.get_reg(2));
                self.pc += 1;
            }

            // clu ra,rb (unsigned compare, set C if ra < rb)
            0x1E => {
                // clu r0,r0
                self.c = false;
                self.pc += 1;
            }
            0x1F => {
                // clu r0,r1
                self.c = self.get_reg(0) < self.get_reg(1);
                self.pc += 1;
            }
            0x20 => {
                // clu r0,r2
                self.c = self.get_reg(0) < self.get_reg(2);
                self.pc += 1;
            }

            // ceq ra,rb (equality compare)
            0x15 => {
                // ceq r0,z
                self.c = self.get_reg(0) == 0;
                self.pc += 1;
            }
            0x16 => {
                // ceq r0,r1
                self.c = self.get_reg(0) == self.get_reg(1);
                self.pc += 1;
            }

            // bra imm8 (unconditional branch)
            0x13 => {
                let imm = self.read_byte(self.pc + 1);
                let next = Self::mask24(self.pc + 2);
                self.pc = Self::mask24(next.wrapping_add(Self::sign_ext8(imm)));
            }

            // brf imm8 (branch if false)
            0x14 => {
                let imm = self.read_byte(self.pc + 1);
                let next = Self::mask24(self.pc + 2);
                if !self.c {
                    self.pc = Self::mask24(next.wrapping_add(Self::sign_ext8(imm)));
                } else {
                    self.pc = next;
                }
            }

            // brt imm8 (branch if true)
            0x12 => {
                let imm = self.read_byte(self.pc + 1);
                let next = Self::mask24(self.pc + 2);
                if self.c {
                    self.pc = Self::mask24(next.wrapping_add(Self::sign_ext8(imm)));
                } else {
                    self.pc = next;
                }
            }

            // push/pop
            0x6C => {
                // push fp
                let sp = self.get_reg(4).wrapping_sub(3);
                self.set_reg(4, sp);
                let v = self.get_reg(3);
                self.write_byte(sp, (v & 0xFF) as u8);
                self.write_byte(sp + 1, ((v >> 8) & 0xFF) as u8);
                self.write_byte(sp + 2, ((v >> 16) & 0xFF) as u8);
                self.pc += 1;
            }
            0x74 => {
                // pop fp
                let sp = self.get_reg(4);
                let b0 = self.read_byte(sp) as u32;
                let b1 = self.read_byte(sp + 1) as u32;
                let b2 = self.read_byte(sp + 2) as u32;
                self.set_reg(3, b0 | (b1 << 8) | (b2 << 16));
                self.set_reg(4, sp.wrapping_add(3));
                self.pc += 1;
            }

            // mov fp,sp and mov sp,fp
            0x4C => {
                // mov fp,sp
                self.set_reg(3, self.get_reg(4));
                self.pc += 1;
            }
            0x53 => {
                // mov sp,fp
                self.set_reg(4, self.get_reg(3));
                self.pc += 1;
            }

            // add sp,imm
            0x21 => {
                let imm = self.read_byte(self.pc + 1);
                let v = Self::mask24(self.get_reg(4).wrapping_add(Self::sign_ext8(imm)));
                self.set_reg(4, v);
                self.pc = Self::mask24(self.pc + 2);
            }

            _ => {
                // Unknown opcode - halt
                eprintln!("Unknown opcode 0x{:02X} at PC=0x{:04X}", b0, self.pc);
                self.halted = true;
                return false;
            }
        }

        true
    }

    pub fn run(&mut self, max_steps: u32) -> u32 {
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            if !self.step() {
                break;
            }
            steps += 1;
        }
        steps
    }
}

/// COR24 Assembler
pub struct Assembler {
    labels: HashMap<String, u32>,
    output: Vec<u8>,
    errors: Vec<String>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            labels: HashMap::new(),
            output: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn assemble(&mut self, source: &str) -> Result<Vec<u8>, Vec<String>> {
        self.labels.clear();
        self.output.clear();
        self.errors.clear();

        // Two passes
        // Pass 1: collect labels
        let mut addr = 0u32;
        for line in source.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(label) = line.strip_suffix(':') {
                self.labels.insert(label.trim().to_string(), addr);
                continue;
            }
            addr += self.estimate_size(line);
        }

        // Pass 2: generate code
        for line in source.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() || line.ends_with(':') {
                continue;
            }
            if let Err(e) = self.emit_instruction(line) {
                self.errors.push(e);
            }
        }

        if self.errors.is_empty() {
            Ok(self.output.clone())
        } else {
            Err(self.errors.clone())
        }
    }

    fn estimate_size(&self, line: &str) -> u32 {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return 0;
        }
        match parts[0].to_lowercase().as_str() {
            "la" | "halt" => 4,
            "lc" | "lcu" | "sb" | "lb" | "lbu" | "sw" | "lw" | "bra" | "brt" | "brf" | "add" => {
                // Check if immediate or register
                if parts.len() > 1 && parts[1].contains(',') {
                    let ops: Vec<&str> = parts[1].split(',').collect();
                    if ops.len() > 1 {
                        let op2 = ops[1].trim();
                        if op2.starts_with("r") || op2 == "c" || op2 == "z" || op2 == "fp" || op2 == "sp" {
                            1 // register form
                        } else {
                            2 // immediate form
                        }
                    } else {
                        2
                    }
                } else {
                    2
                }
            }
            "push" | "pop" | "mov" | "and" | "or" | "xor" | "ceq" | "clu" | "cls" => 1,
            _ => 1,
        }
    }

    fn emit_instruction(&mut self, line: &str) -> Result<(), String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mnemonic = parts[0].to_lowercase();
        let operands: Vec<&str> = if parts.len() > 1 {
            parts[1].split(',').map(|s| s.trim()).collect()
        } else {
            vec![]
        };

        match mnemonic.as_str() {
            "la" => {
                let ra = self.parse_reg(&operands[0])?;
                let imm = self.parse_imm24(&operands[1])?;
                self.output.push(0x29 + ra);
                self.output.push((imm & 0xFF) as u8);
                self.output.push(((imm >> 8) & 0xFF) as u8);
                self.output.push(((imm >> 16) & 0xFF) as u8);
            }
            "lc" => {
                let ra = self.parse_reg(&operands[0])?;
                let imm = self.parse_imm8(&operands[1])?;
                self.output.push(0x44 + ra);
                self.output.push(imm as u8);
            }
            "lcu" => {
                let ra = self.parse_reg(&operands[0])?;
                let imm = self.parse_imm8(&operands[1])?;
                self.output.push(0x48 + ra);
                self.output.push(imm as u8);
            }
            "add" => {
                let ra = self.parse_reg(&operands[0])?;
                let op2 = operands[1].trim();
                if op2.starts_with("r") || op2 == "fp" || op2 == "sp" || op2 == "z" {
                    let rb = self.parse_reg(op2)?;
                    // add ra,rb encoding
                    self.output.push(ra * 8 + rb);
                } else {
                    // add ra,imm
                    let imm = self.parse_imm8(op2)?;
                    self.output.push(0x09 + ra * 8);
                    self.output.push(imm as u8);
                }
            }
            "mov" => {
                let ra = self.parse_reg(&operands[0])?;
                let op2 = operands[1].trim();
                if op2 == "c" {
                    // mov ra,c
                    self.output.push(0x34 + ra * 8);
                } else if op2 == "sp" && ra == 3 {
                    // mov fp,sp
                    self.output.push(0x4C);
                } else if op2 == "fp" && ra == 4 {
                    // mov sp,fp
                    self.output.push(0x53);
                } else {
                    let rb = self.parse_reg(op2)?;
                    self.output.push(0x30 + ra * 8 + rb);
                }
            }
            "and" => {
                let ra = self.parse_reg(&operands[0])?;
                let rb = self.parse_reg(&operands[1])?;
                self.output.push(0x03 + ra * 8 + rb);
            }
            "sb" => {
                // Store byte: sb ra, imm(rb)
                // Encoding: 0x80 + ra*8 + rb
                let ra = self.parse_reg(&operands[0])?;
                let (imm, rb) = self.parse_mem_operand(&operands[1])?;
                self.output.push(0x80 + ra * 8 + rb);
                self.output.push(imm as u8);
            }
            "sw" => {
                // Store word (3 bytes)
                // Encoding: 0x8A + ra (for fp base), or 0x8D + ra*8 + rb (general)
                let ra = self.parse_reg(&operands[0])?;
                let (imm, rb) = self.parse_mem_operand(&operands[1])?;
                if rb == 3 {
                    // fp-based addressing (common for locals)
                    self.output.push(0x8A + ra);
                } else {
                    // general encoding
                    self.output.push(0x8D + ra * 8 + rb);
                }
                self.output.push(imm as u8);
            }
            "lw" => {
                // Load word (3 bytes)
                let ra = self.parse_reg(&operands[0])?;
                let (imm, rb) = self.parse_mem_operand(&operands[1])?;
                if rb == 3 {
                    // fp-based addressing (common for locals)
                    self.output.push(0x92 + ra);
                } else {
                    // general encoding
                    self.output.push(0x95 + ra * 8 + rb);
                }
                self.output.push(imm as u8);
            }
            "ceq" => {
                let ra = self.parse_reg(&operands[0])?;
                let rb = self.parse_reg(&operands[1])?;
                // ceq encoding: base + ra*8 + rb
                self.output.push(0x15 + rb); // simplified
            }
            "clu" => {
                let ra = self.parse_reg(&operands[0])?;
                let rb = self.parse_reg(&operands[1])?;
                self.output.push(0x1E + ra * 3 + rb);
            }
            "bra" => {
                self.output.push(0x13);
                let offset = self.calc_branch_offset(&operands[0])?;
                self.output.push(offset as u8);
            }
            "brf" => {
                self.output.push(0x14);
                let offset = self.calc_branch_offset(&operands[0])?;
                self.output.push(offset as u8);
            }
            "brt" => {
                self.output.push(0x12);
                let offset = self.calc_branch_offset(&operands[0])?;
                self.output.push(offset as u8);
            }
            "push" => {
                let ra = self.parse_reg(&operands[0])?;
                self.output.push(0x64 + ra * 2);
            }
            "pop" => {
                let ra = self.parse_reg(&operands[0])?;
                self.output.push(0x6C + ra * 2);
            }
            "halt" => {
                // Encode as la ir,0
                self.output.push(0xC7);
                self.output.push(0);
                self.output.push(0);
                self.output.push(0);
            }
            _ => {
                return Err(format!("Unknown instruction: {}", mnemonic));
            }
        }
        Ok(())
    }

    fn parse_reg(&self, s: &str) -> Result<u8, String> {
        match s.to_lowercase().as_str() {
            "r0" => Ok(0),
            "r1" => Ok(1),
            "r2" => Ok(2),
            "r3" | "fp" => Ok(3),
            "r4" | "sp" => Ok(4),
            "r5" | "z" => Ok(5),
            "r6" | "iv" => Ok(6),
            "r7" | "ir" => Ok(7),
            _ => Err(format!("Invalid register: {}", s)),
        }
    }

    fn parse_imm8(&self, s: &str) -> Result<i32, String> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            i32::from_str_radix(hex, 16).map_err(|e| e.to_string())
        } else {
            s.parse().map_err(|e: std::num::ParseIntError| e.to_string())
        }
    }

    fn parse_imm24(&self, s: &str) -> Result<u32, String> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u32::from_str_radix(hex, 16).map_err(|e| e.to_string())
        } else {
            let v: i64 = s.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            Ok((v as u32) & 0xFFFFFF)
        }
    }

    fn parse_mem_operand(&self, s: &str) -> Result<(i32, u8), String> {
        if let Some(paren) = s.find('(') {
            let offset_str = &s[..paren];
            let reg_str = s[paren + 1..].trim_end_matches(')');
            let offset = if offset_str.is_empty() {
                0
            } else {
                self.parse_imm8(offset_str)?
            };
            let rb = self.parse_reg(reg_str)?;
            Ok((offset, rb))
        } else {
            Err(format!("Invalid memory operand: {}", s))
        }
    }

    fn calc_branch_offset(&mut self, target: &str) -> Result<i32, String> {
        let target = target.trim();
        let current = self.output.len() as i32 + 1; // +1 for the offset byte itself
        if let Some(&addr) = self.labels.get(target) {
            Ok((addr as i32) - current)
        } else {
            Err(format!("Undefined label: {}", target))
        }
    }
}

/// Print LED state
pub fn print_leds(leds: u8) {
    print!("LEDs: ");
    for i in (0..8).rev() {
        if (leds >> i) & 1 == 1 {
            print!("\x1b[91m●\x1b[0m"); // Red for on
        } else {
            print!("○");
        }
    }
    println!("  (0x{:02X})", leds);
}

/// Full pipeline: compile Rust → WASM → assembly → machine code → run
pub fn run_pipeline(rust_dir: &str, verbose: bool) -> Result<(), String> {
    println!("=== Rust → COR24 Pipeline ===\n");

    // Step 1: Compile Rust to WASM
    println!("Step 1: Compiling Rust to WASM...");
    let output = Command::new("cargo")
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .current_dir(rust_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Cargo build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Find the WASM file
    let wasm_path = format!(
        "{}/target/wasm32-unknown-unknown/release/{}.wasm",
        rust_dir,
        std::path::Path::new(rust_dir)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    );

    if !std::path::Path::new(&wasm_path).exists() {
        return Err(format!("WASM file not found: {}", wasm_path));
    }

    let wasm_size = fs::metadata(&wasm_path).map(|m| m.len()).unwrap_or(0);
    println!("   Generated: {} ({} bytes)\n", wasm_path, wasm_size);

    // Step 2: Translate WASM to COR24 assembly
    println!("Step 2: Translating WASM to COR24 assembly...");
    let wasm_bytes = fs::read(&wasm_path).map_err(|e| format!("Failed to read WASM: {}", e))?;
    let asm = crate::translate_wasm(&wasm_bytes).map_err(|e| format!("Translation failed: {}", e))?;

    let asm_path = format!("{}/output.s", rust_dir);
    fs::write(&asm_path, &asm).map_err(|e| format!("Failed to write assembly: {}", e))?;
    println!("   Generated: {} ({} bytes)\n", asm_path, asm.len());

    if verbose {
        println!("--- Assembly ---");
        println!("{}", asm);
        println!("----------------\n");
    }

    // Step 3: Assemble to machine code
    println!("Step 3: Assembling to machine code...");
    let mut assembler = Assembler::new();
    let machine_code = assembler.assemble(&asm).map_err(|errs| errs.join("\n"))?;
    println!("   Generated: {} bytes of machine code\n", machine_code.len());

    if verbose {
        print!("   Bytes: ");
        for (i, b) in machine_code.iter().enumerate() {
            if i > 0 && i % 16 == 0 {
                print!("\n          ");
            }
            print!("{:02X} ", b);
        }
        println!("\n");
    }

    // Step 4: Run on emulator
    println!("Step 4: Running on COR24 emulator...\n");
    let mut cpu = Cpu::new();
    cpu.load_program(&machine_code);

    let steps = cpu.run(10000);
    println!("Executed {} steps\n", steps);

    // Show LED output
    if !cpu.led_changes.is_empty() {
        println!("LED Changes:");
        for leds in &cpu.led_changes {
            print_leds(*leds);
        }
    }

    println!("\n=== Pipeline Complete ===");
    Ok(())
}
