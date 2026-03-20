//! COR24 assembler
//!
//! Parses COR24 assembly language and produces machine code.
//! Uses encoding tables extracted from the hardware decode ROM.

use crate::cpu::encode;
use crate::cpu::instruction::Opcode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Assembly result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyResult {
    pub bytes: Vec<u8>,
    pub lines: Vec<AssembledLine>,
    pub errors: Vec<String>,
    pub labels: HashMap<String, u32>,
}

/// A single assembled line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledLine {
    pub address: u32,
    pub bytes: Vec<u8>,
    pub source: String,
    pub label: Option<String>,
}

/// COR24 Assembler
pub struct Assembler {
    /// Current address
    address: u32,
    /// Symbol table
    labels: HashMap<String, u32>,
    /// Forward references to resolve
    forward_refs: Vec<ForwardRef>,
    /// Output bytes
    output: Vec<u8>,
    /// Assembled lines
    lines: Vec<AssembledLine>,
    /// Errors
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct ForwardRef {
    address: u32,
    label: String,
    ref_type: RefType,
    line_num: usize,
}

#[derive(Debug, Clone, Copy)]
enum RefType {
    Absolute24, // la instruction
    Relative8,  // branch instruction
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            address: 0,
            labels: HashMap::new(),
            forward_refs: Vec::new(),
            output: Vec::new(),
            lines: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Assemble source code
    pub fn assemble(&mut self, source: &str) -> AssemblyResult {
        self.address = 0;
        self.labels.clear();
        self.forward_refs.clear();
        self.output.clear();
        self.lines.clear();
        self.errors.clear();

        // First pass: collect labels and emit code
        for (line_num, line) in source.lines().enumerate() {
            self.assemble_line(line, line_num);
        }

        // Second pass: resolve forward references
        self.resolve_forward_refs();

        // Update line bytes from resolved output
        for line in &mut self.lines {
            if !line.bytes.is_empty() {
                let addr = line.address as usize;
                let len = line.bytes.len();
                if addr + len <= self.output.len() {
                    line.bytes = self.output[addr..addr + len].to_vec();
                }
            }
        }

        AssemblyResult {
            bytes: self.output.clone(),
            lines: self.lines.clone(),
            errors: self.errors.clone(),
            labels: self.labels.clone(),
        }
    }

    fn assemble_line(&mut self, line: &str, line_num: usize) {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            return;
        }

        // Strip trailing comments before any parsing (colons in comments
        // would otherwise be misidentified as label separators)
        let line = if let Some(pos) = line.find(';') {
            line[..pos].trim()
        } else if let Some(pos) = line.find('#') {
            line[..pos].trim()
        } else {
            line
        };

        if line.is_empty() {
            return;
        }

        let start_addr = self.address;
        let mut label = None;
        let mut instruction_part = line;

        // Check for label
        if let Some(colon_pos) = line.find(':') {
            let label_str = line[..colon_pos].trim();
            if !label_str.is_empty() {
                label = Some(label_str.to_string());
                self.labels.insert(label_str.to_string(), self.address);
            }
            instruction_part = line[colon_pos + 1..].trim();
        }

        // Handle directives
        if instruction_part.starts_with('.') {
            let before_len = self.output.len();
            self.handle_directive(instruction_part, line_num);
            // Record directive bytes in lines so they appear in output
            let directive_bytes = self.output[before_len..].to_vec();
            if !directive_bytes.is_empty() || label.is_some() {
                self.lines.push(AssembledLine {
                    address: start_addr,
                    bytes: directive_bytes,
                    source: line.to_string(),
                    label,
                });
            }
            return;
        }

        // Skip if no instruction
        if instruction_part.is_empty() {
            if label.is_some() {
                self.lines.push(AssembledLine {
                    address: start_addr,
                    bytes: vec![],
                    source: line.to_string(),
                    label,
                });
            }
            return;
        }

        // Parse instruction
        let bytes = self.parse_instruction(instruction_part, line_num);

        self.lines.push(AssembledLine {
            address: start_addr,
            bytes: bytes.clone(),
            source: line.to_string(),
            label,
        });

        for b in bytes {
            self.output.push(b);
            self.address += 1;
        }
    }

    fn handle_directive(&mut self, directive: &str, _line_num: usize) {
        let parts: Vec<&str> = directive.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0].to_lowercase().as_str() {
            ".org" => {
                if parts.len() > 1
                    && let Some(addr) = self.parse_number(parts[1])
                {
                    // Pad output to reach the new address
                    while self.output.len() < addr as usize {
                        self.output.push(0);
                    }
                    self.address = addr;
                }
            }
            ".byte" | ".db" => {
                for part in &parts[1..] {
                    if let Some(val) = self.parse_number(part.trim_matches(',')) {
                        self.output.push(val as u8);
                        self.address += 1;
                    }
                }
            }
            ".word" | ".dw" => {
                for part in &parts[1..] {
                    if let Some(val) = self.parse_number(part.trim_matches(',')) {
                        // 24-bit word, little-endian
                        self.output.push((val & 0xFF) as u8);
                        self.output.push(((val >> 8) & 0xFF) as u8);
                        self.output.push(((val >> 16) & 0xFF) as u8);
                        self.address += 3;
                    }
                }
            }
            ".ascii" | ".asciz" => {
                // Extract string between quotes
                if let Some(start) = directive.find('"')
                    && let Some(end) = directive.rfind('"')
                    && end > start
                {
                    let s = &directive[start + 1..end];
                    for c in s.bytes() {
                        self.output.push(c);
                        self.address += 1;
                    }
                    if parts[0].to_lowercase() == ".asciz" {
                        self.output.push(0);
                        self.address += 1;
                    }
                }
            }
            ".comm" => {
                // BSS allocation: .comm symbol, size
                // Defines label at current address and advances by size
                // No bytes emitted (memory is zero-initialized)
                if parts.len() >= 3 {
                    let sym = parts[1].trim_matches(',');
                    if let Some(size) = self.parse_number(parts[2].trim_matches(',')) {
                        self.labels.insert(sym.to_string(), self.address);
                        self.address += size;
                    }
                }
            }
            // Directives that are safe to ignore in flat memory model
            ".text" | ".data" | ".globl" | ".global" | ".section" | ".align" => {}
            _ => {}
        }
    }

    fn parse_instruction(&mut self, inst: &str, line_num: usize) -> Vec<u8> {
        // Strip trailing comments
        let inst = if let Some(pos) = inst.find(';') {
            &inst[..pos]
        } else if let Some(pos) = inst.find('#') {
            &inst[..pos]
        } else {
            inst
        };

        let parts: Vec<&str> = inst.split_whitespace().collect();
        if parts.is_empty() {
            return vec![];
        }

        let mnemonic = parts[0].to_lowercase();
        let operands_str = if parts.len() > 1 {
            parts[1..].join(" ")
        } else {
            String::new()
        };
        let operands: Vec<&str> = if !operands_str.is_empty() {
            operands_str.split(',').map(|s| s.trim()).collect()
        } else {
            vec![]
        };

        match mnemonic.as_str() {
            // Stack operations
            "push" => self.encode_push(&operands, line_num),
            "pop" => self.encode_pop(&operands, line_num),

            // Move operations
            "mov" => self.encode_mov(&operands, line_num),

            // Arithmetic
            "add" => self.encode_add(&operands, line_num),
            "sub" => self.encode_sub(&operands, line_num),
            "mul" => self.encode_mul(&operands, line_num),

            // Logic
            "and" => self.encode_alu(&operands, Opcode::And, "and", line_num),
            "or" => self.encode_alu(&operands, Opcode::Or, "or", line_num),
            "xor" => self.encode_alu(&operands, Opcode::Xor, "xor", line_num),

            // Shifts
            "shl" => self.encode_alu(&operands, Opcode::Shl, "shl", line_num),
            "sra" => self.encode_alu(&operands, Opcode::Sra, "sra", line_num),
            "srl" => self.encode_alu(&operands, Opcode::Srl, "srl", line_num),

            // Compares
            "ceq" => self.encode_alu(&operands, Opcode::Ceq, "ceq", line_num),
            "cls" => self.encode_alu(&operands, Opcode::Cls, "cls", line_num),
            "clu" => self.encode_alu(&operands, Opcode::Clu, "clu", line_num),

            // Branches
            "bra" => self.encode_branch(&operands, Opcode::Bra, line_num),
            "brf" => self.encode_branch(&operands, Opcode::Brf, line_num),
            "brt" => self.encode_branch(&operands, Opcode::Brt, line_num),

            // Jumps
            "jmp" => self.encode_jmp(&operands, line_num),
            "jal" => self.encode_jal(&operands, line_num),

            // Load operations
            "la" => self.encode_la(&operands, line_num),
            "lc" => self.encode_lc(&operands, false, line_num),
            "lcu" => self.encode_lc(&operands, true, line_num),
            "lb" => self.encode_load_store(&operands, Opcode::Lb, "lb", line_num),
            "lbu" => self.encode_load_store(&operands, Opcode::Lbu, "lbu", line_num),
            "lw" => self.encode_load_store(&operands, Opcode::Lw, "lw", line_num),

            // Store operations
            "sb" => self.encode_load_store(&operands, Opcode::Sb, "sb", line_num),
            "sw" => self.encode_load_store(&operands, Opcode::Sw, "sw", line_num),

            // Extensions
            "sxt" => self.encode_alu(&operands, Opcode::Sxt, "sxt", line_num),
            "zxt" => self.encode_alu(&operands, Opcode::Zxt, "zxt", line_num),

            // Pseudo-instructions
            "nop" => vec![0xFF],

            _ => {
                self.errors.push(format!(
                    "Line {}: Unknown instruction '{}'",
                    line_num + 1,
                    mnemonic
                ));
                vec![]
            }
        }
    }

    fn parse_register(&self, s: &str) -> Option<u8> {
        let s = s.trim().to_lowercase();
        match s.as_str() {
            "r0" => Some(0),
            "r1" => Some(1),
            "r2" => Some(2),
            "fp" => Some(3),
            "sp" => Some(4),
            "z" | "c" => Some(5), // z=zero for comparisons, c=condition flag for mov
            "iv" => Some(6),
            "ir" => Some(7),
            _ => None,
        }
    }

    fn parse_number(&self, s: &str) -> Option<u32> {
        let s = s.trim();
        if s.starts_with("0x") || s.starts_with("0X") {
            // C-style hex: 0xFF
            u32::from_str_radix(&s[2..], 16).ok()
        } else if (s.ends_with('h') || s.ends_with('H'))
            && s.len() > 1
            && s.as_bytes()[0].is_ascii_hexdigit()
        {
            // Intel-style hex: 0FFh, 2Ah (must start with digit)
            u32::from_str_radix(&s[..s.len() - 1], 16).ok()
        } else if s.starts_with('-') {
            s.parse::<i32>().ok().map(|v| v as u32)
        } else {
            s.parse::<u32>().ok()
        }
    }

    fn encode_push(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.is_empty() {
            self.errors
                .push(format!("Line {}: push requires operand", line_num + 1));
            return vec![];
        }

        if let Some(ra) = self.parse_register(operands[0]) {
            if let Some(byte) = encode::encode_push(ra) {
                vec![byte]
            } else {
                self.errors.push(format!(
                    "Line {}: push {} not supported",
                    line_num + 1,
                    operands[0]
                ));
                vec![]
            }
        } else {
            self.errors.push(format!(
                "Line {}: Invalid register '{}'",
                line_num + 1,
                operands[0]
            ));
            vec![]
        }
    }

    fn encode_pop(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.is_empty() {
            self.errors
                .push(format!("Line {}: pop requires operand", line_num + 1));
            return vec![];
        }

        if let Some(ra) = self.parse_register(operands[0]) {
            if let Some(byte) = encode::encode_pop(ra) {
                vec![byte]
            } else {
                self.errors.push(format!(
                    "Line {}: pop {} not supported",
                    line_num + 1,
                    operands[0]
                ));
                vec![]
            }
        } else {
            self.errors.push(format!(
                "Line {}: Invalid register '{}'",
                line_num + 1,
                operands[0]
            ));
            vec![]
        }
    }

    fn encode_mov(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors
                .push(format!("Line {}: mov requires two operands", line_num + 1));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let rb = self.parse_register(operands[1]);

        match (ra, rb) {
            (Some(ra), Some(rb)) => {
                if let Some(byte) = encode::encode_mov(ra, rb) {
                    vec![byte]
                } else {
                    self.errors.push(format!(
                        "Line {}: mov {},{} not supported",
                        line_num + 1,
                        operands[0],
                        operands[1]
                    ));
                    vec![]
                }
            }
            _ => {
                self.errors
                    .push(format!("Line {}: Invalid mov operands", line_num + 1));
                vec![]
            }
        }
    }

    fn encode_add(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors
                .push(format!("Line {}: add requires two operands", line_num + 1));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);

        // Check if second operand is immediate
        if let Some(imm) = self.parse_number(operands[1]) {
            // add ra,imm
            if let Some(ra) = ra {
                if let Some(byte) = encode::encode_add_imm(ra) {
                    vec![byte, imm as u8]
                } else {
                    self.errors.push(format!(
                        "Line {}: add {},imm not supported",
                        line_num + 1,
                        operands[0]
                    ));
                    vec![]
                }
            } else {
                self.errors
                    .push(format!("Line {}: Invalid register", line_num + 1));
                vec![]
            }
        } else if let Some(rb) = self.parse_register(operands[1]) {
            // add ra,rb
            if let Some(ra) = ra {
                if let Some(byte) = encode::encode_add_reg(ra, rb) {
                    vec![byte]
                } else {
                    self.errors.push(format!(
                        "Line {}: add {},{} not supported",
                        line_num + 1,
                        operands[0],
                        operands[1]
                    ));
                    vec![]
                }
            } else {
                self.errors
                    .push(format!("Line {}: Invalid register", line_num + 1));
                vec![]
            }
        } else {
            self.errors
                .push(format!("Line {}: Invalid add operand", line_num + 1));
            vec![]
        }
    }

    fn encode_sub(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors
                .push(format!("Line {}: sub requires two operands", line_num + 1));
            return vec![];
        }

        // Check for sub sp,imm24 pattern
        if operands[0].to_lowercase() == "sp"
            && let Some(imm) = self.parse_number(operands[1])
            && let Some(byte) = encode::encode_sub_sp()
        {
            return vec![
                byte,
                (imm & 0xFF) as u8,
                ((imm >> 8) & 0xFF) as u8,
                ((imm >> 16) & 0xFF) as u8,
            ];
        }

        // sub ra,rb
        let ra = self.parse_register(operands[0]);
        let rb = self.parse_register(operands[1]);

        match (ra, rb) {
            (Some(ra), Some(rb)) => {
                if let Some(byte) = encode::encode_instruction(Opcode::Sub, ra, rb) {
                    vec![byte]
                } else {
                    self.errors.push(format!(
                        "Line {}: sub {},{} not supported",
                        line_num + 1,
                        operands[0],
                        operands[1]
                    ));
                    vec![]
                }
            }
            _ => {
                self.errors
                    .push(format!("Line {}: Invalid sub operands", line_num + 1));
                vec![]
            }
        }
    }

    fn encode_mul(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors
                .push(format!("Line {}: mul requires two operands", line_num + 1));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let rb = self.parse_register(operands[1]);

        match (ra, rb) {
            (Some(ra), Some(rb)) => {
                if let Some(byte) = encode::encode_instruction(Opcode::Mul, ra, rb) {
                    vec![byte]
                } else {
                    self.errors.push(format!(
                        "Line {}: mul {},{} not supported",
                        line_num + 1,
                        operands[0],
                        operands[1]
                    ));
                    vec![]
                }
            }
            _ => {
                self.errors
                    .push(format!("Line {}: Invalid mul operands", line_num + 1));
                vec![]
            }
        }
    }

    /// Generic ALU instruction encoding (and, or, xor, shifts, compares, etc.)
    fn encode_alu(
        &mut self,
        operands: &[&str],
        opcode: Opcode,
        mnemonic: &str,
        line_num: usize,
    ) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors.push(format!(
                "Line {}: {} requires two operands",
                line_num + 1,
                mnemonic
            ));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let rb = self.parse_register(operands[1]);

        match (ra, rb) {
            (Some(ra), Some(rb)) => {
                if let Some(byte) = encode::encode_instruction(opcode, ra, rb) {
                    vec![byte]
                } else {
                    self.errors.push(format!(
                        "Line {}: {} {},{} not supported",
                        line_num + 1,
                        mnemonic,
                        operands[0],
                        operands[1]
                    ));
                    vec![]
                }
            }
            _ => {
                self.errors.push(format!(
                    "Line {}: Invalid {} operands",
                    line_num + 1,
                    mnemonic
                ));
                vec![]
            }
        }
    }

    fn encode_branch(&mut self, operands: &[&str], opcode: Opcode, line_num: usize) -> Vec<u8> {
        if operands.is_empty() {
            self.errors
                .push(format!("Line {}: branch requires target", line_num + 1));
            return vec![];
        }

        let target = operands[0].trim();
        let first_byte = encode::encode_branch(opcode).unwrap_or(0x13);

        // Check if it's a label
        if let Some(&addr) = self.labels.get(target) {
            // Calculate relative offset: COR24 pipeline means branch base = instr_addr + 4
            let branch_base = self.address + 4;
            let offset = (addr as i32) - (branch_base as i32);
            if !(-128..=127).contains(&offset) {
                self.errors
                    .push(format!("Line {}: Branch target too far", line_num + 1));
                return vec![];
            }
            vec![first_byte, offset as u8]
        } else if let Some(imm) = self.parse_number(target) {
            // Direct offset
            vec![first_byte, imm as u8]
        } else {
            // Forward reference
            self.forward_refs.push(ForwardRef {
                address: self.address + 1,
                label: target.to_string(),
                ref_type: RefType::Relative8,
                line_num,
            });
            vec![first_byte, 0x00] // Placeholder
        }
    }

    fn encode_jmp(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.is_empty() {
            self.errors
                .push(format!("Line {}: jmp requires target", line_num + 1));
            return vec![];
        }

        let target = operands[0].trim();

        // Check for (ra) syntax - indirect jump
        if target.starts_with('(') && target.ends_with(')') {
            let reg = &target[1..target.len() - 1];
            if let Some(ra) = self.parse_register(reg) {
                if let Some(byte) = encode::encode_jmp(ra) {
                    return vec![byte];
                } else {
                    self.errors.push(format!(
                        "Line {}: jmp ({}) not supported",
                        line_num + 1,
                        reg
                    ));
                    return vec![];
                }
            }
        }

        self.errors
            .push(format!("Line {}: Invalid jmp syntax", line_num + 1));
        vec![]
    }

    fn encode_jal(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors
                .push(format!("Line {}: jal requires ra,(rb)", line_num + 1));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let rb_str = operands[1].trim();

        // Parse (rb) syntax
        if rb_str.starts_with('(') && rb_str.ends_with(')') {
            let reg = &rb_str[1..rb_str.len() - 1];
            if let (Some(ra), Some(rb)) = (ra, self.parse_register(reg)) {
                if let Some(byte) = encode::encode_jal(ra, rb) {
                    return vec![byte];
                } else {
                    self.errors.push(format!(
                        "Line {}: jal {},({}) not supported",
                        line_num + 1,
                        operands[0],
                        reg
                    ));
                    return vec![];
                }
            }
        }

        self.errors
            .push(format!("Line {}: Invalid jal syntax", line_num + 1));
        vec![]
    }

    fn encode_la(&mut self, operands: &[&str], line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors.push(format!(
                "Line {}: la requires register and address",
                line_num + 1
            ));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let target = operands[1].trim();

        if let Some(ra) = ra {
            if let Some(first_byte) = encode::encode_la(ra) {
                if let Some(addr) = self.parse_number(target) {
                    // Immediate address
                    return vec![
                        first_byte,
                        (addr & 0xFF) as u8,
                        ((addr >> 8) & 0xFF) as u8,
                        ((addr >> 16) & 0xFF) as u8,
                    ];
                } else if let Some(&addr) = self.labels.get(target) {
                    // Known label
                    return vec![
                        first_byte,
                        (addr & 0xFF) as u8,
                        ((addr >> 8) & 0xFF) as u8,
                        ((addr >> 16) & 0xFF) as u8,
                    ];
                } else {
                    // Forward reference
                    self.forward_refs.push(ForwardRef {
                        address: self.address + 1,
                        label: target.to_string(),
                        ref_type: RefType::Absolute24,
                        line_num,
                    });
                    return vec![first_byte, 0x00, 0x00, 0x00];
                }
            } else {
                self.errors.push(format!(
                    "Line {}: la {} not supported",
                    line_num + 1,
                    operands[0]
                ));
                return vec![];
            }
        }

        self.errors
            .push(format!("Line {}: Invalid la operand", line_num + 1));
        vec![]
    }

    fn encode_lc(&mut self, operands: &[&str], unsigned: bool, line_num: usize) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors.push(format!(
                "Line {}: lc requires register and constant",
                line_num + 1
            ));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let imm = self.parse_number(operands[1]);

        match (ra, imm) {
            (Some(ra), Some(imm)) => {
                // lc sign-extends an 8-bit immediate: valid range is 0..=127
                // (values 128..=255 get sign-extended to large negative numbers)
                if !unsigned && imm > 127 && imm < 0xFFFFFF80 {
                    self.errors.push(format!(
                        "Line {}: lc immediate {} out of range (0..127 or use lcu for unsigned)",
                        line_num + 1, imm
                    ));
                    return vec![];
                }
                if unsigned && imm > 255 {
                    self.errors.push(format!(
                        "Line {}: lcu immediate {} out of range (0..255)",
                        line_num + 1, imm
                    ));
                    return vec![];
                }
                if let Some(first_byte) = encode::encode_lc(ra, unsigned) {
                    vec![first_byte, imm as u8]
                } else {
                    let mnemonic = if unsigned { "lcu" } else { "lc" };
                    self.errors.push(format!(
                        "Line {}: {} {} not supported",
                        line_num + 1,
                        mnemonic,
                        operands[0]
                    ));
                    vec![]
                }
            }
            _ => {
                self.errors
                    .push(format!("Line {}: Invalid lc operands", line_num + 1));
                vec![]
            }
        }
    }

    fn encode_load_store(
        &mut self,
        operands: &[&str],
        opcode: Opcode,
        mnemonic: &str,
        line_num: usize,
    ) -> Vec<u8> {
        if operands.len() < 2 {
            self.errors.push(format!(
                "Line {}: {} requires operands",
                line_num + 1,
                mnemonic
            ));
            return vec![];
        }

        let ra = self.parse_register(operands[0]);
        let addr_part = operands[1].trim();

        // Parse offset(rb) syntax
        if let Some(paren_pos) = addr_part.find('(') {
            let offset_str = &addr_part[..paren_pos];
            let rb_str = &addr_part[paren_pos + 1..].trim_end_matches(')');

            let offset = if offset_str.is_empty() {
                Some(0)
            } else {
                self.parse_number(offset_str)
            };
            let rb = self.parse_register(rb_str);

            if let (Some(ra), Some(rb), Some(offset)) = (ra, rb, offset) {
                if let Some(first_byte) = encode::encode_load_store(opcode, ra, rb) {
                    return vec![first_byte, offset as u8];
                } else {
                    self.errors.push(format!(
                        "Line {}: {} {},{} not supported",
                        line_num + 1,
                        mnemonic,
                        operands[0],
                        operands[1]
                    ));
                    return vec![];
                }
            }
        }

        self.errors.push(format!(
            "Line {}: Invalid {} syntax",
            line_num + 1,
            mnemonic
        ));
        vec![]
    }

    fn resolve_forward_refs(&mut self) {
        for fref in &self.forward_refs {
            if let Some(&target_addr) = self.labels.get(&fref.label) {
                match fref.ref_type {
                    RefType::Absolute24 => {
                        let addr = fref.address as usize;
                        if addr + 2 < self.output.len() {
                            self.output[addr] = (target_addr & 0xFF) as u8;
                            self.output[addr + 1] = ((target_addr >> 8) & 0xFF) as u8;
                            self.output[addr + 2] = ((target_addr >> 16) & 0xFF) as u8;
                        }
                    }
                    RefType::Relative8 => {
                        // fref.address is the offset byte (instr_addr + 1)
                        // branch_base = instr_addr + 4 = fref.address + 3
                        let branch_base = fref.address + 3;
                        let offset = (target_addr as i32) - (branch_base as i32);
                        if (-128..=127).contains(&offset) {
                            let addr = fref.address as usize;
                            if addr < self.output.len() {
                                self.output[addr] = offset as u8;
                            }
                        } else {
                            self.errors.push(format!(
                                "Line {}: Branch target '{}' too far",
                                fref.line_num + 1,
                                fref.label
                            ));
                        }
                    }
                }
            } else {
                self.errors.push(format!(
                    "Line {}: Undefined label '{}'",
                    fref.line_num + 1,
                    fref.label
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assembly() {
        let mut asm = Assembler::new();
        let result = asm.assemble("lc r0,42");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x44, 42]);
    }

    #[test]
    fn test_push_pop() {
        let mut asm = Assembler::new();
        let result = asm.assemble("push r0\npop r1");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x7D, 0x7A]);
    }

    #[test]
    fn test_add_register() {
        let mut asm = Assembler::new();
        let result = asm.assemble("add r0,r1\nadd r1,r2");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x01, 0x05]);
    }

    #[test]
    fn test_add_immediate() {
        let mut asm = Assembler::new();
        let result = asm.assemble("add r0,10");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x09, 10]);
    }

    #[test]
    fn test_mov() {
        let mut asm = Assembler::new();
        let result = asm.assemble("mov fp,sp\nmov sp,fp");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x65, 0x69]);
    }

    #[test]
    fn test_load_word() {
        let mut asm = Assembler::new();
        let result = asm.assemble("lw r0,4(fp)");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x4D, 4]);
    }

    #[test]
    fn test_store_word() {
        let mut asm = Assembler::new();
        let result = asm.assemble("sw r1,8(fp)");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0xAA, 8]);
    }

    #[test]
    fn test_sub_register() {
        let mut asm = Assembler::new();
        let result = asm.assemble("sub r0,r1");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x9C]);
    }

    #[test]
    fn test_mul() {
        let mut asm = Assembler::new();
        let result = asm.assemble("mul r0,r1");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x6B]);
    }

    #[test]
    fn test_logic_ops() {
        let mut asm = Assembler::new();
        let result = asm.assemble("and r0,r1\nor r1,r2\nxor r0,r2");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x0D, 0x76, 0xB9]);
    }

    #[test]
    fn test_shifts() {
        let mut asm = Assembler::new();
        let result = asm.assemble("shl r0,r1\nsra r1,r0\nsrl r2,r1");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x8A, 0x92, 0x9B]);
    }

    #[test]
    fn test_compare() {
        let mut asm = Assembler::new();
        let result = asm.assemble("ceq r0,r1\ncls r1,r0");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x16, 0x1B]);
    }

    #[test]
    fn test_branch() {
        let mut asm = Assembler::new();
        let result = asm.assemble("bra 10\nbrf -5\nbrt 0");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x13, 10, 0x14, 0xFB, 0x15, 0]);
    }

    #[test]
    fn test_jmp() {
        let mut asm = Assembler::new();
        let result = asm.assemble("jmp (r0)\njmp (r1)");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x26, 0x27]);
    }

    #[test]
    fn test_jal() {
        let mut asm = Assembler::new();
        let result = asm.assemble("jal r1,(r0)");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x25]);
    }

    #[test]
    fn test_la() {
        let mut asm = Assembler::new();
        let result = asm.assemble("la r0,0x1234");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x29, 0x34, 0x12, 0x00]);
    }

    #[test]
    fn test_extensions() {
        let mut asm = Assembler::new();
        let result = asm.assemble("sxt r0,r1\nzxt r1,r2");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0xB0, 0xC3]);
    }

    #[test]
    fn test_trailing_comments() {
        let mut asm = Assembler::new();
        let result = asm.assemble("lc r0,10       ; load constant\nadd r0,r1  # add");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes, vec![0x44, 10, 0x01]);
    }

    #[test]
    fn test_led_blink_integration() {
        use crate::cpu::{CpuState, Executor};

        // Minimal LED blink test: load LED address, write value, halt
        let code = r#"
            la      r0, 0xFF0000    ; LED I/O address
            lc      r1, 0x55        ; Value to write
            sb      r1, 0(r0)       ; Write to LEDs
        "#;

        let mut asm = Assembler::new();
        let result = asm.assemble(code);
        assert!(result.errors.is_empty(), "Assembly errors: {:?}", result.errors);

        let mut cpu = CpuState::new();
        cpu.load_program(0, &result.bytes);

        let executor = Executor::new();

        // Execute the 3 instructions
        for i in 0..3 {
            let res = executor.step(&mut cpu);
            assert!(matches!(res, crate::cpu::ExecuteResult::Ok), "Step {} failed: {:?}", i, res);
        }

        // Check LED value
        assert_eq!(cpu.io.leds, 0x55, "LED value should be 0x55, got 0x{:02X}", cpu.io.leds);
    }

    #[test]
    fn test_branch_backward_label() {
        // Backward branch: offset = target - (branch_instr_addr + 4)
        // addr 0: lc r0,5    (2 bytes)
        // addr 2: loop:
        // addr 2: add r0,r0  (1 byte)
        // addr 3: bra loop   (2 bytes) -> offset = 2 - (3+4) = -5
        let mut asm = Assembler::new();
        let result = asm.assemble("lc r0,5\nloop: add r0,r0\nbra loop");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes[3], 0x13); // bra
        assert_eq!(result.bytes[4] as i8, -5); // offset = 2 - 7 = -5
    }

    #[test]
    fn test_branch_forward_label() {
        // Forward branch: offset = target - (branch_instr_addr + 4)
        // addr 0: bra skip   (2 bytes) -> offset = 3 - (0+4) = -1
        // addr 2: add r0,r0  (1 byte)
        // addr 3: skip:
        // addr 3: lc r0,1    (2 bytes)
        let mut asm = Assembler::new();
        let result = asm.assemble("bra skip\nadd r0,r0\nskip: lc r0,1");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.bytes[0], 0x13); // bra
        assert_eq!(result.bytes[1] as i8, -1); // offset = 3 - 4 = -1
    }

    #[test]
    fn test_branch_loop_integration() {
        use crate::cpu::{CpuState, Executor};

        // Count down from 3 to 0 using branch loop:
        //   lc r0,3          ; r0 = 3
        // loop:
        //   add r0,-1        ; r0 -= 1  (add imm with sign-extended 0xFF = -1)
        //   ceq r0,z         ; compare r0 to zero
        //   brf loop         ; if not equal, loop
        let code = "lc r0,3\nloop: add r0,0xFF\nceq r0,z\nbrf loop";
        let mut asm = Assembler::new();
        let result = asm.assemble(code);
        assert!(result.errors.is_empty(), "Assembly errors: {:?}", result.errors);

        let mut cpu = CpuState::new();
        cpu.load_program(0, &result.bytes);
        let executor = Executor::new();

        // Should run: lc, then 3 iterations of (add, ceq, brf), then fall through
        // = 1 + 3*3 = 10 instructions, but brf not taken on last = still 10
        executor.run(&mut cpu, 100);

        assert_eq!(cpu.get_reg(0), 0, "r0 should be 0 after counting down");
        assert!(cpu.c, "c flag should be true (r0 == z)");
    }

    #[test]
    fn test_example10_button_echo() {
        let code = r#"; Example 10: Button Echo
; LED D2 lights when button S2 is pressed

        la      r1,0xFF0000 ; I/O address (LEDSWDAT)
        lc      r2,1        ; Bit mask for XOR

loop:
        lb      r0,0(r1)    ; Read button S2
        xor     r0,r2       ; Invert
        sb      r0,0(r1)    ; Write to LED D2

        bra     loop        ; Keep polling

halt:   bra     halt        ; Never reached
"#;
        let mut asm = Assembler::new();
        let result = asm.assemble(code);
        assert!(result.errors.is_empty(), "Assembly errors: {:?}", result.errors);
        assert!(!result.bytes.is_empty(), "Should produce bytes");
    }
}
