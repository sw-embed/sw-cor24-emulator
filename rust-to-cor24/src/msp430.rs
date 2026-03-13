//! MSP430 assembly to COR24 assembly translator
//!
//! Translates MSP430 assembly (as emitted by `rustc --target msp430-none-elf --emit asm`)
//! into COR24 assembly that can be assembled by the existing COR24 assembler.
//!
//! ## Register Mapping
//!
//! MSP430 calling convention uses r12-r14 for args, r12 for return.
//! COR24 has only 3 GP registers (r0-r2).
//!
//! | MSP430 | COR24 | Role            |
//! |--------|-------|-----------------|
//! | r12    | r0    | arg0 / return   |
//! | r13    | r1    | arg1            |
//! | r14    | r2    | arg2            |
//! | r1     | sp    | stack pointer   |
//! | r0     | (PC)  | implicit        |
//! | r2     | (SR)  | status register |
//! | r3     | (CG)  | constant gen    |
//! | r4-r11 | stack | spilled to fp-relative |

use anyhow::{Result, bail};

/// A parsed MSP430 assembly line
#[derive(Debug, Clone)]
enum MspLine {
    /// Assembly directive (.section, .globl, .type, .size, .p2align, .ident, .file)
    Directive(String),
    /// Label definition (e.g., "add:", ".LBB0_1:")
    Label(String),
    /// Instruction with mnemonic and operands
    Instruction(MspInst),
    /// Comment or blank line
    Comment(String),
}

/// A parsed MSP430 instruction
#[derive(Debug, Clone)]
struct MspInst {
    mnemonic: String,
    byte_mode: bool, // .b suffix (e.g., and.b)
    operands: Vec<MspOperand>,
}

/// MSP430 operand types
#[derive(Debug, Clone)]
enum MspOperand {
    /// Register: r0-r15
    Register(u8),
    /// Immediate: #value
    Immediate(i32),
    /// Symbolic/label: &label or just label (for call/jmp targets)
    Symbol(String),
    /// Indexed: offset(Rn)
    Indexed(i32, u8),
    /// Indirect register: @Rn
    Indirect(u8),
    /// Indirect autoincrement: @Rn+
    IndirectAutoInc(()),
    /// Absolute: &addr
    Absolute(()),
}

/// Translate MSP430 assembly text to COR24 assembly text.
///
/// Looks for the `entry_point` function (default: `"start"`) and emits a
/// `bra <entry>` reset vector prologue at address 0. If the input has `.globl`
/// directives (i.e., comes from rustc) but the entry label is missing, returns
/// an error. For simple inputs without `.globl` (hand-written fragments), no
/// prologue is emitted.
pub fn translate_msp430(msp_asm: &str, entry_point: &str) -> Result<String> {
    let lines = parse_msp430(msp_asm)?;

    // Collect all labels for validation
    let all_labels: Vec<&str> = lines.iter().filter_map(|l| {
        if let MspLine::Label(name) = l { Some(name.as_str()) } else { None }
    }).collect();

    let has_globl = lines.iter().any(|l| matches!(l, MspLine::Directive(d) if d.starts_with(".globl")));
    let has_entry = all_labels.contains(&entry_point);

    // If this is compiled code (.globl present) but entry label is missing, fail
    if has_globl && !has_entry {
        let available: Vec<_> = all_labels.iter()
            .filter(|l| !l.starts_with('.') && !l.starts_with("_RN"))
            .cloned()
            .collect();
        bail!("entry point '{}' not found. COR24 convention requires a \
               #[no_mangle] pub unsafe fn {}() entry point.\n  \
               Available labels: {}",
            entry_point, entry_point, available.join(", "));
    }

    let mut out = String::new();
    out.push_str("; COR24 Assembly - Generated from MSP430 via msp430-to-cor24\n");
    out.push_str("; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM\n\n");

    // Emit reset vector: initialize frame pointer and jump to entry point
    // Uses la+jmp instead of bra because bra has ±127 byte range limit
    // fp must be initialized so spill slots (positive fp offsets) point to
    // valid writable memory above the stack (stack grows downward from sp)
    if has_entry {
        out.push_str(&format!("; Reset vector -> {}\n", entry_point));
        out.push_str("    mov     fp, sp\n");
        out.push_str(&format!("    la      r0, {}\n", entry_point));
        out.push_str("    jmp     (r0)\n\n");
    }

    // Track which functions we're in for context
    let mut in_text_section = false;
    let mut _current_func: Option<String> = None;
    let mut call_label_counter: usize = 0;

    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        match line {
            MspLine::Comment(c) => {
                // @cor24: passthrough — emit raw COR24 assembly from asm!() blocks
                if let Some(raw) = c.strip_prefix("; @cor24: ").or_else(|| c.strip_prefix(";@cor24: ")) {
                    let raw = raw.trim();
                    // Labels end with ':', emit without indentation
                    if raw.ends_with(':') {
                        out.push_str(&format!("{}\n", raw));
                    } else {
                        out.push_str(&format!("    {}\n", raw));
                    }
                } else if c == ";APP" || c == ";NO_APP" {
                    // Skip rustc inline asm markers
                } else if !c.is_empty() {
                    out.push_str(&format!("; {}\n", c));
                } else {
                    out.push('\n');
                }
            }
            MspLine::Directive(d) => {
                if d.starts_with(".section") && d.contains(".text") {
                    in_text_section = true;
                    if let Some(name) = d.strip_prefix(".section\t.text.") {
                        let name = name.split(',').next().unwrap_or(name);
                        _current_func = Some(name.to_string());
                        out.push_str(&format!("; --- function: {} ---\n", name));
                    }
                } else if d.starts_with(".section") {
                    in_text_section = false;
                }
            }
            MspLine::Label(label) => {
                if !in_text_section {
                    i += 1;
                    continue;
                }
                out.push_str(&format!("{}:\n", label));
            }
            MspLine::Instruction(inst) => {
                if !in_text_section {
                    i += 1;
                    continue;
                }
                // Tail call optimization: call + ret -> direct jump
                if inst.mnemonic == "call" && is_followed_by_ret(&lines, i) {
                    match translate_tail_call(inst) {
                        Ok(cor24_lines) => {
                            for cl in cor24_lines {
                                out.push_str(&format!("    {}\n", cl));
                            }
                        }
                        Err(e) => {
                            out.push_str(&format!("    ; TODO: tail {} ({})\n", inst.mnemonic, e));
                        }
                    }
                    // Skip past the ret instruction
                    i = skip_to_ret(&lines, i) + 1;
                    continue;
                }
                // Compare look-ahead: cmp followed by jeq/jne needs ceq, not clu
                if inst.mnemonic == "cmp" {
                    let use_ceq = is_followed_by_equality_branch(&lines, i);
                    match translate_cmp(&inst.operands, inst.byte_mode, use_ceq) {
                        Ok(cor24_lines) => {
                            for cl in cor24_lines {
                                out.push_str(&format!("    {}\n", cl));
                            }
                        }
                        Err(e) => {
                            out.push_str(&format!("    ; TODO: {} ({})\n", inst.mnemonic, e));
                        }
                    }
                    i += 1;
                    continue;
                }
                match translate_instruction(inst, &mut call_label_counter) {
                    Ok(cor24_lines) => {
                        for cl in cor24_lines {
                            out.push_str(&format!("    {}\n", cl));
                        }
                    }
                    Err(e) => {
                        out.push_str(&format!("    ; TODO: {} ({})\n", inst.mnemonic, e));
                    }
                }
            }
        }
        i += 1;
    }

    Ok(out)
}

/// Map MSP430 register number to COR24 register name.
/// r12 -> r0, r13 -> r1, r14 -> r2
/// r1 (SP) -> sp
/// r4-r15 -> spill slots accessed via fp-relative offsets
///
/// For spilled registers, we return a COR24 GP register (r0-r2) that
/// will be used as a proxy. The caller must load from/store to the
/// spill slot as appropriate. For simple operand mapping in most
/// instructions, we use r0 as the working register and emit
/// load/store around it.
fn map_register(msp_reg: u8) -> Result<String> {
    match msp_reg {
        12 => Ok("r0".to_string()),
        13 => Ok("r1".to_string()),
        14 => Ok("r2".to_string()),
        1 => Ok("sp".to_string()),
        // Spilled registers: 4-11, 15
        // These need special handling - return a marker
        r @ (4..=11 | 15) => Ok(format!("spill_{}", r)),
        _ => bail!("register r{} not mappable", msp_reg),
    }
}

/// Check if a mapped register name is a spill slot
fn is_spill(reg: &str) -> bool {
    reg.starts_with("spill_")
}

/// Get the spill slot offset for a spilled MSP430 register.
/// Spill slots are at fp-relative offsets: r4 -> 0(fp), r5 -> 3(fp), etc.
/// We use 3-byte slots since COR24 is 24-bit.
fn spill_offset(reg: &str) -> u8 {
    if let Some(num_str) = reg.strip_prefix("spill_") {
        let msp_reg: u8 = num_str.parse().unwrap_or(4);
        let slot = match msp_reg {
            4 => 0,
            5 => 1,
            6 => 2,
            7 => 3,
            8 => 4,
            9 => 5,
            10 => 6,
            11 => 7,
            15 => 8,
            _ => 0,
        };
        slot * 3
    } else {
        0
    }
}

/// Load a potentially-spilled register into a COR24 GP register.
/// If the register is already a GP register, returns it as-is.
/// If it's a spill slot, emits a load instruction and returns the working register.
fn load_spill(result: &mut Vec<String>, reg: &str, working: &str) -> String {
    if is_spill(reg) {
        let off = spill_offset(reg);
        result.push(format!("lw      {}, {}(fp)", working, off));
        working.to_string()
    } else {
        reg.to_string()
    }
}

/// Store a COR24 GP register back to a spill slot if needed.
fn store_spill(result: &mut Vec<String>, reg: &str, working: &str) {
    if is_spill(reg) {
        let off = spill_offset(reg);
        result.push(format!("sw      {}, {}(fp)", working, off));
    }
}

/// Map MSP430 register to COR24 for use as load/store base register.
/// COR24 only allows r0, r1, r2, fp as base registers (not sp).
/// When MSP430 uses SP (r1) as base, we use a temp register to avoid
/// clobbering fp (which may hold the spill frame pointer).
/// The `avoid` parameter specifies registers that shouldn't be used as base.
fn map_base_register(msp_reg: u8, result: &mut Vec<String>, avoid: &str) -> Result<String> {
    match msp_reg {
        1 => {
            // SP can't be used as base in COR24 load/store.
            // Copy sp to a temp register and use that as base.
            let base = if avoid == "r2" { "r1" } else { "r2" };
            result.push(format!("mov     {}, sp", base));
            Ok(base.to_string())
        }
        _ => map_register(msp_reg),
    }
}

/// Translate a single MSP430 instruction to one or more COR24 instructions.
fn translate_instruction(inst: &MspInst, call_counter: &mut usize) -> Result<Vec<String>> {
    let mn = inst.mnemonic.as_str();
    let ops = &inst.operands;

    match mn {
        // --- Arithmetic ---
        "add" => translate_binary_op("add", ops, inst.byte_mode),
        "sub" => translate_sub(ops, inst.byte_mode),
        "and" => translate_binary_op("and", ops, inst.byte_mode),
        "bis" => translate_binary_op("or", ops, inst.byte_mode),  // BIS = bit set = OR
        "bic" => translate_bic(ops),
        "xor" => translate_binary_op("xor", ops, inst.byte_mode),

        // --- Moves ---
        "mov" => translate_mov(ops, inst.byte_mode),
        "clr" => translate_clr(ops),

        // --- Compare ---
        "cmp" => translate_cmp(ops, inst.byte_mode, false),
        "tst" => translate_tst(ops),
        "bit" => translate_bit(ops),

        // --- Shifts ---
        "rra" => translate_shift_right(ops, true),   // arithmetic
        "rrc" => translate_shift_right(ops, false),  // through carry (logical-ish)
        "clrc" => Ok(vec!["; clrc (clear carry - handled by shift sequence)".to_string()]),

        // --- Branches ---
        "jmp" => translate_branch("bra", ops),
        "jnz" | "jne" => translate_cond_branch(true, ops),
        "jz" | "jeq" => translate_cond_branch(false, ops),
        "jlo" => translate_branch("brt", ops),
        "jhs" | "jc" => translate_branch("brf", ops),
        "jge" => translate_branch("brf", ops),
        "jl" => translate_branch("brt", ops),
        "jn" => translate_branch("brt", ops),

        // --- Call/Return ---
        "call" => translate_call(ops, call_counter),
        "ret" => Ok(translate_ret()),

        // --- Stack ---
        "push" => translate_push(ops),
        "pop" => translate_pop(ops),

        // --- Increment/Decrement ---
        "inc" => {
            let dst = operand_to_reg(&ops[0])?;
            let mut result = Vec::new();
            if is_spill(&dst) {
                result.push("push    r0".to_string());
                let actual = load_spill(&mut result, &dst, "r0");
                result.push(format!("add     {}, 1", actual));
                store_spill(&mut result, &dst, "r0");
                result.push("pop     r0".to_string());
            } else {
                result.push(format!("add     {}, 1", dst));
            }
            Ok(result)
        }
        "dec" => {
            let dst = operand_to_reg(&ops[0])?;
            let mut result = Vec::new();
            if is_spill(&dst) {
                result.push("push    r0".to_string());
                let actual = load_spill(&mut result, &dst, "r0");
                result.push(format!("add     {}, -1", actual));
                store_spill(&mut result, &dst, "r0");
                result.push("pop     r0".to_string());
            } else {
                result.push(format!("add     {}, -1", dst));
            }
            Ok(result)
        }

        // --- Special ---
        "nop" => Ok(vec!["nop".to_string()]),

        _ => bail!("unsupported mnemonic: {}", mn),
    }
}

/// Translate binary operations (add, and, or, xor) where src can be reg or imm.
/// When spill registers are involved, push/pop the working register to avoid
/// clobbering live GP values (stack-machine style).
fn translate_binary_op(cor24_op: &str, ops: &[MspOperand], byte_mode: bool) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("{} requires 2 operands", cor24_op);
    }
    let dst_raw = operand_to_reg(&ops[1])?;
    let mut result = Vec::new();

    // Handle spilled destination: push/save working reg, load spill, operate, store, pop/restore
    let (dst, dst_is_spill, dst_working) = if is_spill(&dst_raw) {
        // Pick a working register that doesn't conflict with the source GP register
        let w = match &ops[0] {
            MspOperand::Register(r) => {
                match map_register(*r).unwrap_or_default().as_str() {
                    "r0" => "r1",
                    "r1" => "r0",
                    _ => "r0",
                }
            }
            _ => "r0",
        };
        result.push(format!("push    {}", w));
        let actual = load_spill(&mut result, &dst_raw, w);
        (actual, true, w.to_string())
    } else {
        (dst_raw.clone(), false, String::new())
    };

    match &ops[0] {
        MspOperand::Register(r) => {
            let src_raw = map_register(*r)?;
            if is_spill(&src_raw) {
                // Load spilled source into a different working register, with push/pop
                let w = if dst == "r0" { "r1" } else { "r0" };
                result.push(format!("push    {}", w));
                let src = load_spill(&mut result, &src_raw, w);
                result.push(format!("{:<8}{}, {}", cor24_op, dst, src));
                result.push(format!("pop     {}", w));
            } else {
                result.push(format!("{:<8}{}, {}", cor24_op, dst, src_raw));
            }
        }
        MspOperand::Immediate(imm) => {
            if cor24_op == "add" {
                let val = if byte_mode { *imm & 0xFF } else { *imm };
                // Scale SP adjustments: MSP430 2-byte words → COR24 3-byte words
                let val = if dst == "sp" { val * 3 / 2 } else { val };
                if (-128..=127).contains(&val) {
                    result.push(format!("add     {}, {}", dst, val));
                } else {
                    let tmp = temp_reg(&dst);
                    result.push(format!("la      {}, 0x{:06X}", tmp, val as u32 & 0xFFFFFF));
                    result.push(format!("add     {}, {}", dst, tmp));
                }
            } else {
                let tmp = temp_reg(&dst);
                load_immediate(&mut result, &tmp, *imm);
                result.push(format!("{:<8}{}, {}", cor24_op, dst, tmp));
            }
        }
        _ => bail!("unsupported source operand for {}", cor24_op),
    }

    if byte_mode && dst != "sp" {
        let tmp = temp_reg(&dst);
        result.push(format!("lcu     {}, 0xFF", tmp));
        result.push(format!("and     {}, {}", dst, tmp));
    }

    // Store back to spill slot and restore working register
    if dst_is_spill {
        store_spill(&mut result, &dst_raw, &dst);
        result.push(format!("pop     {}", dst_working));
    }

    Ok(result)
}

/// Translate SUB - MSP430 sub is dst = dst - src.
/// Push/pop working registers to avoid clobbering live GP values.
fn translate_sub(ops: &[MspOperand], byte_mode: bool) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("sub requires 2 operands");
    }
    let dst_raw = operand_to_reg(&ops[1])?;
    let mut result = Vec::new();

    let (dst, dst_is_spill, dst_working) = if is_spill(&dst_raw) {
        let w = match &ops[0] {
            MspOperand::Register(r) => {
                match map_register(*r).unwrap_or_default().as_str() {
                    "r0" => "r1",
                    "r1" => "r0",
                    _ => "r0",
                }
            }
            _ => "r0",
        };
        result.push(format!("push    {}", w));
        let actual = load_spill(&mut result, &dst_raw, w);
        (actual, true, w.to_string())
    } else {
        (dst_raw.clone(), false, String::new())
    };

    match &ops[0] {
        MspOperand::Register(r) => {
            let src_raw = map_register(*r)?;
            if is_spill(&src_raw) {
                let w = if dst == "r0" { "r1" } else { "r0" };
                result.push(format!("push    {}", w));
                let src = load_spill(&mut result, &src_raw, w);
                result.push(format!("sub     {}, {}", dst, src));
                result.push(format!("pop     {}", w));
            } else {
                result.push(format!("sub     {}, {}", dst, src_raw));
            }
        }
        MspOperand::Immediate(imm) => {
            let neg = -*imm;
            if dst == "sp" {
                let scaled = *imm * 3 / 2;
                result.push(format!("sub     sp, {}", scaled));
            } else if (-128..=127).contains(&neg) {
                result.push(format!("add     {}, {}", dst, neg));
            } else {
                let tmp = temp_reg(&dst);
                load_immediate(&mut result, &tmp, *imm);
                result.push(format!("sub     {}, {}", dst, tmp));
            }
        }
        _ => bail!("unsupported source operand for sub"),
    }

    if byte_mode && dst != "sp" {
        let tmp = temp_reg(&dst);
        load_immediate(&mut result, &tmp, 0xFF);
        result.push(format!("and     {}, {}", dst, tmp));
    }

    if dst_is_spill {
        store_spill(&mut result, &dst_raw, &dst);
        result.push(format!("pop     {}", dst_working));
    }

    Ok(result)
}

/// BIC = bit clear: dst &= ~src
fn translate_bic(ops: &[MspOperand]) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("bic requires 2 operands");
    }
    // BIC src, dst -> dst = dst AND NOT(src)
    // COR24 doesn't have NOT, so we XOR with 0xFFFFFF then AND
    let dst = operand_to_reg(&ops[1])?;
    let mut result = Vec::new();

    match &ops[0] {
        MspOperand::Immediate(imm) => {
            // Compute ~imm at translate time
            let inverted = !(*imm) & 0xFFFFFF;
            let tmp = temp_reg(&dst);
            load_immediate(&mut result, &tmp, inverted);
            result.push(format!("and     {}, {}", dst, tmp));
        }
        MspOperand::Register(r) => {
            let src = map_register(*r)?;
            let tmp = temp_reg(&dst);
            // tmp = 0xFFFFFF
            result.push(format!("la      {}, 0xFFFFFF", tmp));
            // tmp = tmp XOR src = ~src
            result.push(format!("xor     {}, {}", tmp, src));
            // dst = dst AND tmp
            result.push(format!("and     {}, {}", dst, tmp));
        }
        _ => bail!("unsupported source for bic"),
    }

    Ok(result)
}

/// Translate MOV instruction - covers many MSP430 patterns
fn translate_mov(ops: &[MspOperand], byte_mode: bool) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("mov requires 2 operands");
    }
    let mut result = Vec::new();

    match (&ops[0], &ops[1]) {
        // mov Rsrc, Rdst -> register to register
        (MspOperand::Register(src), MspOperand::Register(dst)) => {
            let s_raw = map_register(*src)?;
            let d_raw = map_register(*dst)?;
            if s_raw == d_raw && !is_spill(&s_raw) {
                // Same register, no-op
            } else if is_spill(&s_raw) && is_spill(&d_raw) {
                // Both spilled: push/pop r0 to avoid clobbering
                result.push("push    r0".to_string());
                load_spill(&mut result, &s_raw, "r0");
                store_spill(&mut result, &d_raw, "r0");
                result.push("pop     r0".to_string());
            } else if is_spill(&s_raw) {
                // Source spilled: load into destination (destination IS being written)
                let d = &d_raw;
                load_spill(&mut result, &s_raw, d);
            } else if is_spill(&d_raw) {
                // Destination spilled: store source into spill slot (no temp needed)
                let s = &s_raw;
                store_spill(&mut result, &d_raw, s);
            } else {
                result.push(format!("mov     {}, {}", d_raw, s_raw));
            }
        }
        // mov #imm, Rdst -> load immediate
        (MspOperand::Immediate(imm), MspOperand::Register(dst)) => {
            let d_raw = map_register(*dst)?;
            // Check if this is an MSP430 I/O address that needs 24-bit mapping
            let mapped_imm = map_io_address_imm(*imm);
            if is_spill(&d_raw) {
                // Push/pop r0 to avoid clobbering live value
                result.push("push    r0".to_string());
                load_immediate(&mut result, "r0", mapped_imm);
                store_spill(&mut result, &d_raw, "r0");
                result.push("pop     r0".to_string());
            } else {
                load_immediate(&mut result, &d_raw, mapped_imm);
            }
        }
        // mov Rsrc, offset(Rdst) -> store word/byte
        (MspOperand::Register(src), MspOperand::Indexed(off, dst)) => {
            let s_raw = map_register(*src)?;
            let s = if is_spill(&s_raw) {
                load_spill(&mut result, &s_raw, "r0");
                "r0".to_string()
            } else {
                s_raw
            };
            let d = map_base_register(*dst, &mut result, &s)?;
            let store_op = if byte_mode { "sb" } else { "sw" };
            result.push(format!("{}      {}, {}({})", store_op, s, off, d));
        }
        // mov offset(Rsrc), Rdst -> load word/byte
        (MspOperand::Indexed(off, src), MspOperand::Register(dst)) => {
            let d_raw_preview = map_register(*dst)?;
            let avoid = if is_spill(&d_raw_preview) { "r0" } else { &d_raw_preview };
            let s = map_base_register(*src, &mut result, avoid)?;
            let d_raw = map_register(*dst)?;
            let load_op = if byte_mode { "lbu" } else { "lw" };
            if is_spill(&d_raw) {
                result.push(format!("{}      r0, {}({})", load_op, off, s));
                store_spill(&mut result, &d_raw, "r0");
            } else {
                result.push(format!("{}      {}, {}({})", load_op, d_raw, off, s));
            }
        }
        // mov @Rsrc, Rdst -> load indirect
        (MspOperand::Indirect(src), MspOperand::Register(dst)) => {
            let s = map_register(*src)?;
            let d = map_register(*dst)?;
            let load_op = if byte_mode { "lbu" } else { "lw" };
            result.push(format!("{}      {}, 0({})", load_op, d, s));
        }
        // mov #imm, offset(Rdst) -> store immediate to memory
        (MspOperand::Immediate(imm), MspOperand::Indexed(off, dst)) => {
            let d = map_register(*dst)?;
            let tmp = temp_reg(&d);
            load_immediate(&mut result, &tmp, *imm);
            let store_op = if byte_mode { "sb" } else { "sw" };
            result.push(format!("{}      {}, {}({})", store_op, tmp, off, d));
        }
        // mov #symbol, Rdst -> load address
        (MspOperand::Symbol(sym), MspOperand::Register(dst)) => {
            let d = map_register(*dst)?;
            result.push(format!("la      {}, {}", d, sym));
        }
        _ => bail!("unsupported mov operand combination: {:?} -> {:?}", ops[0], ops[1]),
    }

    Ok(result)
}

/// CLR Rdst -> lc Rdst, 0
fn translate_clr(ops: &[MspOperand]) -> Result<Vec<String>> {
    let dst = operand_to_reg(&ops[0])?;
    if is_spill(&dst) {
        let mut result = Vec::new();
        result.push("push    r0".to_string());
        result.push("lc      r0, 0".to_string());
        store_spill(&mut result, &dst, "r0");
        result.push("pop     r0".to_string());
        Ok(result)
    } else {
        Ok(vec![format!("lc      {}, 0", dst)])
    }
}

/// CMP src, dst -> sets condition flags
/// MSP430 CMP semantics: computes dst - src, sets flags.
/// We emit both ceq and clu depending on what we can determine.
/// Since we can't know the following branch at this point, we emit clu
/// which works for most patterns (jlo/jhs). For jeq/jne, we use ceq.
///
/// COR24 ceq constraints: only (r0,r1), (r0,r2), (r0,z), (r1,r2), (r1,z), (r2,z)
/// COR24 clu constraints: all combos of r0,r1,r2 + (z,r0), (z,r1), (z,r2)
fn translate_cmp(ops: &[MspOperand], byte_mode: bool, use_ceq: bool) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("cmp requires 2 operands");
    }
    let mut result = Vec::new();
    let dst_raw = operand_to_reg(&ops[1])?;

    // Handle spilled destination — push/pop to avoid clobbering
    let (dst, dst_saved) = if is_spill(&dst_raw) {
        result.push("push    r0".to_string());
        load_spill(&mut result, &dst_raw, "r0");
        ("r0".to_string(), true)
    } else {
        (dst_raw, false)
    };

    // If byte mode, mask dst to 8 bits first
    if byte_mode {
        let tmp = temp_reg(&dst);
        result.push(format!("lcu     {}, 0xFF", tmp));
        result.push(format!("and     {}, {}", dst, tmp));
    }

    match &ops[0] {
        MspOperand::Register(r) => {
            let src_raw = map_register(*r)?;
            if is_spill(&src_raw) {
                let w = if dst == "r0" { "r1" } else { "r0" };
                result.push(format!("push    {}", w));
                load_spill(&mut result, &src_raw, w);
                if use_ceq {
                    let (a, b) = order_ceq_operands(&dst, w);
                    result.push(format!("ceq     {}, {}", a, b));
                } else {
                    result.push(format!("clu     {}, {}", dst, w));
                }
                result.push(format!("pop     {}", w));
            } else if use_ceq {
                let (a, b) = order_ceq_operands(&dst, &src_raw);
                result.push(format!("ceq     {}, {}", a, b));
            } else {
                result.push(format!("clu     {}, {}", dst, src_raw));
            }
        }
        MspOperand::Immediate(imm) => {
            if *imm == 0 {
                result.push(format!("ceq     {}, z", dst));
            } else {
                let tmp = temp_reg(&dst);
                result.push(format!("push    {}", tmp));
                load_immediate(&mut result, &tmp, *imm);
                if use_ceq || *imm == -1 {
                    let (a, b) = order_ceq_operands(&dst, &tmp);
                    result.push(format!("ceq     {}, {}", a, b));
                } else {
                    result.push(format!("clu     {}, {}", dst, tmp));
                }
                result.push(format!("pop     {}", tmp));
            }
        }
        _ => bail!("unsupported cmp source"),
    }

    if dst_saved {
        result.push("pop     r0".to_string());
    }

    Ok(result)
}

/// Order operands for ceq to satisfy COR24 hardware constraints.
/// ceq only supports: (r0,r1), (r0,r2), (r0,z), (r1,r2), (r1,z), (r2,z)
/// Since equality is commutative, we just put the smaller-numbered register first.
fn order_ceq_operands<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    let rank = |r: &str| -> u8 {
        match r {
            "r0" => 0,
            "r1" => 1,
            "r2" => 2,
            "z" => 5,
            _ => 3,
        }
    };
    if rank(a) <= rank(b) { (a, b) } else { (b, a) }
}

/// TST Rdst -> ceq Rdst, z
fn translate_tst(ops: &[MspOperand]) -> Result<Vec<String>> {
    let dst = operand_to_reg(&ops[0])?;
    if is_spill(&dst) {
        let mut result = Vec::new();
        result.push("push    r0".to_string());
        let actual = load_spill(&mut result, &dst, "r0");
        result.push(format!("ceq     {}, z", actual));
        result.push("pop     r0".to_string());
        Ok(result)
    } else {
        Ok(vec![format!("ceq     {}, z", dst)])
    }
}

/// BIT src, dst -> test bits (AND without storing, sets flags)
fn translate_bit(ops: &[MspOperand]) -> Result<Vec<String>> {
    if ops.len() != 2 {
        bail!("bit requires 2 operands");
    }
    let dst = operand_to_reg(&ops[1])?;
    let mut result = Vec::new();

    // BIT is like AND but doesn't store result, just sets flags
    // We need a temp to avoid destroying dst
    let tmp = temp_reg(&dst);
    result.push(format!("mov     {}, {}", tmp, dst));

    match &ops[0] {
        MspOperand::Immediate(imm) => {
            let tmp2 = temp_reg2(&dst, &tmp);
            load_immediate(&mut result, &tmp2, *imm);
            result.push(format!("and     {}, {}", tmp, tmp2));
        }
        MspOperand::Register(r) => {
            let src = map_register(*r)?;
            result.push(format!("and     {}, {}", tmp, src));
        }
        _ => bail!("unsupported bit source"),
    }
    result.push(format!("ceq     {}, z", tmp));

    Ok(result)
}

/// Translate shift right (RRA = arithmetic, RRC = through carry)
fn translate_shift_right(ops: &[MspOperand], arithmetic: bool) -> Result<Vec<String>> {
    let dst = operand_to_reg(&ops[0])?;
    let tmp = temp_reg(&dst);
    let mut result = Vec::new();
    // COR24 shift right: sra (arithmetic) or srl (logical)
    // MSP430 RRA/RRC shifts by 1 bit
    result.push(format!("lc      {}, 1", tmp));
    if arithmetic {
        result.push(format!("sra     {}, {}", dst, tmp));
    } else {
        result.push(format!("srl     {}, {}", dst, tmp));
    }
    Ok(result)
}

/// Translate unconditional/named branch
fn translate_branch(cor24_branch: &str, ops: &[MspOperand]) -> Result<Vec<String>> {
    match &ops[0] {
        MspOperand::Symbol(target) => {
            Ok(vec![format!("{:<8}{}", cor24_branch, target)])
        }
        _ => bail!("branch target must be a label"),
    }
}

/// Translate conditional branches.
/// MSP430 jeq/jz = jump if zero flag set (after tst/cmp)
/// MSP430 jne/jnz = jump if zero flag clear
///
/// COR24: after ceq, c=1 means equal. brt = branch if c=1, brf = branch if c=0.
/// So: jeq (jump if equal/zero) -> brt
///     jne (jump if not equal/not zero) -> brf
fn translate_cond_branch(is_jne: bool, ops: &[MspOperand]) -> Result<Vec<String>> {
    let target = match &ops[0] {
        MspOperand::Symbol(s) => s.clone(),
        _ => bail!("branch target must be a label"),
    };

    if is_jne {
        // jne/jnz: branch if NOT equal -> brf (c=0 means not equal after ceq)
        Ok(vec![format!("brf     {}", target)])
    } else {
        // jeq/jz: branch if equal/zero -> brt (c=1 means equal after ceq)
        Ok(vec![format!("brt     {}", target)])
    }
}

/// Translate CALL instruction.
///
/// MSP430 `call` pushes return address to stack and jumps; all GP registers
/// are preserved. COR24 `jal` stores return address in a register (clobbering it).
///
/// To match MSP430 semantics, we emulate call with stack-based return:
///   la  r2, .Lret_N     ; compute return address
///   push r2              ; push to stack (like MSP430 call)
///   la  r2, target       ; load target
///   jmp (r2)             ; jump (preserves r0, r1)
/// .Lret_N:
///
/// This preserves r0 (arg0) and r1 (arg1) across the call.
/// r2 (arg2) is clobbered but MSP430 r14 is caller-saved.
fn translate_call(ops: &[MspOperand], call_counter: &mut usize) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let ret_label = format!(".Lret_{}", call_counter);
    *call_counter += 1;

    match &ops[0] {
        MspOperand::Symbol(target) => {
            result.push(format!("; call {}", target));
            result.push(format!("la      r2, {}", ret_label));
            result.push("push    r2".to_string());
            result.push(format!("la      r2, {}", target));
            result.push("jmp     (r2)".to_string());
            result.push(format!("{}:", ret_label));
        }
        MspOperand::Register(r) => {
            let src = map_register(*r)?;
            result.push(format!("la      r2, {}", ret_label));
            result.push("push    r2".to_string());
            if src != "r2" {
                result.push(format!("mov     r2, {}", src));
            }
            result.push("jmp     (r2)".to_string());
            result.push(format!("{}:", ret_label));
        }
        _ => bail!("unsupported call operand"),
    }

    Ok(result)
}

/// Translate RET -> pop return address from stack and jump to it.
/// Matches MSP430 semantics where return address was pushed by call.
fn translate_ret() -> Vec<String> {
    vec![
        "pop     r2".to_string(),
        "jmp     (r2)".to_string(),
    ]
}

/// Check if the instruction at `call_idx` is immediately followed by `ret`
/// (skipping comments). Labels or other instructions break the pattern.
fn is_followed_by_ret(lines: &[MspLine], call_idx: usize) -> bool {
    for j in (call_idx + 1)..lines.len() {
        match &lines[j] {
            MspLine::Comment(_) => continue,
            MspLine::Instruction(inst) if inst.mnemonic == "ret" => return true,
            _ => return false,
        }
    }
    false
}

/// Check if a `cmp` is followed by `jeq`/`jne` (equality branch, needs ceq)
/// vs `jhs`/`jlo` (unsigned branch, needs clu).
fn is_followed_by_equality_branch(lines: &[MspLine], cmp_idx: usize) -> bool {
    for j in (cmp_idx + 1)..lines.len() {
        match &lines[j] {
            MspLine::Comment(_) => continue,
            MspLine::Instruction(inst) => {
                return matches!(inst.mnemonic.as_str(), "jeq" | "jne" | "jz" | "jnz");
            }
            _ => return false,
        }
    }
    false
}

/// Find the index of the `ret` following a call. Assumes `is_followed_by_ret` was true.
fn skip_to_ret(lines: &[MspLine], call_idx: usize) -> usize {
    for j in (call_idx + 1)..lines.len() {
        if let MspLine::Instruction(inst) = &lines[j] {
            if inst.mnemonic == "ret" {
                return j;
            }
        }
    }
    call_idx
}

/// Translate a tail call: `call #target` where next instruction is `ret`.
/// Instead of push return addr / call / pop / ret, just jump to target.
/// The callee returns directly to our caller (whose address is already on stack).
fn translate_tail_call(inst: &MspInst) -> Result<Vec<String>> {
    let mut result = Vec::new();
    match &inst.operands[0] {
        MspOperand::Symbol(target) => {
            result.push(format!("; tail call {}", target));
            result.push(format!("la      r2, {}", target));
            result.push("jmp     (r2)".to_string());
        }
        MspOperand::Register(r) => {
            let src = map_register(*r)?;
            result.push("; tail call (indirect)".to_string());
            if src != "r2" {
                result.push(format!("mov     r2, {}", src));
            }
            result.push("jmp     (r2)".to_string());
        }
        _ => bail!("unsupported tail call operand"),
    }
    Ok(result)
}

/// Translate PUSH
fn translate_push(ops: &[MspOperand]) -> Result<Vec<String>> {
    match &ops[0] {
        MspOperand::Register(r) => {
            let reg = map_register(*r)?;
            if is_spill(&reg) {
                // Spilled register: load from spill slot, then push.
                // Use r1 as working register to avoid clobbering r0 (arg0/return value)
                let mut result = Vec::new();
                load_spill(&mut result, &reg, "r1");
                result.push("push    r1".to_string());
                Ok(result)
            } else {
                match reg.as_str() {
                    "r0" | "r1" | "r2" | "fp" => {
                        Ok(vec![format!("push    {}", reg)])
                    }
                    "sp" => {
                        Ok(vec!["; push sp (skipped)".to_string()])
                    }
                    _ => bail!("can't push {}", reg),
                }
            }
        }
        _ => bail!("push requires register operand"),
    }
}

/// Translate POP
fn translate_pop(ops: &[MspOperand]) -> Result<Vec<String>> {
    match &ops[0] {
        MspOperand::Register(r) => {
            let reg = map_register(*r)?;
            if is_spill(&reg) {
                // Spilled register: pop into r1, store to spill slot.
                // Use r1 as working register to avoid clobbering r0 (return value)
                let mut result = Vec::new();
                result.push("pop     r1".to_string());
                store_spill(&mut result, &reg, "r1");
                Ok(result)
            } else {
                match reg.as_str() {
                    "r0" | "r1" | "r2" | "fp" => {
                        Ok(vec![format!("pop     {}", reg)])
                    }
                    _ => bail!("can't pop {}", reg),
                }
            }
        }
        _ => bail!("pop requires register operand"),
    }
}

// --- Helper functions ---

/// Get a COR24 register name from an operand that should be a register
fn operand_to_reg(op: &MspOperand) -> Result<String> {
    match op {
        MspOperand::Register(r) => map_register(*r),
        _ => bail!("expected register operand, got {:?}", op),
    }
}

/// Map MSP430 16-bit I/O addresses to COR24 24-bit I/O addresses.
/// MSP430 uses 16-bit addresses (0xFF00-0xFF02) that sign-extend to wrong 24-bit values.
/// Convention: MSP430 addr 0xFFXX → COR24 addr 0xFFXX00 (shift left 8 bits).
/// Only applies to the known I/O address range (0xFF00-0xFF02, i.e., -256 to -254).
fn map_io_address_imm(val: i32) -> i32 {
    // Check for MSP430 I/O addresses: -256 (0xFF00), -255 (0xFF01), -254 (0xFF02)
    if (-256..=-254).contains(&val) {
        let u16val = (val as u16) as u32;  // 0xFF00, 0xFF01, 0xFF02
        (u16val << 8) as i32              // 0xFF0000, 0xFF0100, 0xFF0200
    } else {
        val
    }
}

/// Load an immediate value into a COR24 register
fn load_immediate(result: &mut Vec<String>, reg: &str, value: i32) {
    if value == 0 {
        result.push(format!("lc      {}, 0", reg));
    } else if (-128..=127).contains(&value) {
        result.push(format!("lc      {}, {}", reg, value));
    } else {
        result.push(format!("la      {}, 0x{:06X}", reg, value as u32 & 0xFFFFFF));
    }
}

/// Pick a temp register that isn't `avoid`
fn temp_reg(avoid: &str) -> String {
    match avoid {
        "r0" => "r1".to_string(),
        _ => "r0".to_string(),
    }
}

/// Pick a temp register that isn't `avoid1` or `avoid2`
fn temp_reg2(avoid1: &str, avoid2: &str) -> String {
    for r in &["r0", "r1", "r2"] {
        if *r != avoid1 && *r != avoid2 {
            return r.to_string();
        }
    }
    "r2".to_string()
}

// ==========================================
// MSP430 Assembly Parser
// ==========================================

/// Parse MSP430 assembly text into structured lines
fn parse_msp430(asm: &str) -> Result<Vec<MspLine>> {
    let mut lines = Vec::new();

    for raw_line in asm.lines() {
        let line = raw_line.trim();

        // Empty line
        if line.is_empty() {
            lines.push(MspLine::Comment(String::new()));
            continue;
        }

        // Full-line comment
        if line.starts_with(';') || line.starts_with('#') {
            lines.push(MspLine::Comment(line.to_string()));
            continue;
        }

        // Label (identifier followed by colon) — check before directives
        // because local labels like .LBB0_1: start with '.'
        if let Some(label) = line.strip_suffix(':') {
            if !label.contains(char::is_whitespace) {
                lines.push(MspLine::Label(label.to_string()));
                continue;
            }
        }

        // Directive (starts with '.' but NOT a label)
        if line.starts_with('.') {
            lines.push(MspLine::Directive(line.to_string()));
            continue;
        }

        // Check for label: instruction on same line
        if let Some(colon_pos) = line.find(':') {
            let before = &line[..colon_pos];
            if !before.contains(char::is_whitespace) && !before.starts_with('#') {
                lines.push(MspLine::Label(before.to_string()));
                let after = line[colon_pos + 1..].trim();
                if !after.is_empty() {
                    if let Some(inst) = parse_instruction(after)? {
                        lines.push(MspLine::Instruction(inst));
                    }
                }
                continue;
            }
        }

        // Instruction
        if let Some(inst) = parse_instruction(line)? {
            lines.push(MspLine::Instruction(inst));
        }
    }

    Ok(lines)
}

/// Parse a single MSP430 instruction line
fn parse_instruction(line: &str) -> Result<Option<MspInst>> {
    // Strip trailing comment
    let line = if let Some(pos) = line.find(';') {
        &line[..pos]
    } else {
        line
    };
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    // Split into mnemonic and operands
    let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
    let mnemonic_raw = parts[0].to_lowercase();

    // Check for .b suffix (byte mode)
    let (mnemonic, byte_mode) = if let Some(base) = mnemonic_raw.strip_suffix(".b") {
        (base.to_string(), true)
    } else if let Some(base) = mnemonic_raw.strip_suffix(".w") {
        (base.to_string(), false) // .w is default
    } else {
        (mnemonic_raw, false)
    };

    // Parse operands
    let operands = if parts.len() > 1 {
        parse_operands(parts[1].trim())?
    } else {
        Vec::new()
    };

    Ok(Some(MspInst {
        mnemonic,
        byte_mode,
        operands,
    }))
}

/// Parse MSP430 operand list (comma-separated)
fn parse_operands(text: &str) -> Result<Vec<MspOperand>> {
    let mut operands = Vec::new();

    // Split by comma, but be careful with parentheses
    for part in split_operands(text) {
        let op = parse_single_operand(part.trim())?;
        operands.push(op);
    }

    Ok(operands)
}

/// Split operands by comma, respecting parentheses
fn split_operands(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;

    for (i, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(&text[start..]);
    result
}

/// Parse a single MSP430 operand
fn parse_single_operand(text: &str) -> Result<MspOperand> {
    let text = text.trim();

    // Immediate: #value or #symbol
    if let Some(rest) = text.strip_prefix('#') {
        if let Ok(val) = parse_number(rest) {
            return Ok(MspOperand::Immediate(val));
        }
        // Could be a symbol like #mmio_write
        return Ok(MspOperand::Symbol(rest.to_string()));
    }

    // Indirect autoincrement: @Rn+
    if text.starts_with('@') && text.ends_with('+') {
        let reg_str = &text[1..text.len() - 1];
        if let Some(_r) = parse_register(reg_str) {
            return Ok(MspOperand::IndirectAutoInc(()));
        }
    }

    // Indirect register: @Rn
    if let Some(rest) = text.strip_prefix('@') {
        if let Some(r) = parse_register(rest) {
            return Ok(MspOperand::Indirect(r));
        }
    }

    // Absolute: &addr or &symbol
    if let Some(rest) = text.strip_prefix('&') {
        if let Ok(_val) = parse_number(rest) {
            return Ok(MspOperand::Absolute(()));
        }
        return Ok(MspOperand::Symbol(rest.to_string()));
    }

    // Indexed: offset(Rn)
    if let Some(paren_pos) = text.find('(') {
        if text.ends_with(')') {
            let offset_str = &text[..paren_pos];
            let reg_str = &text[paren_pos + 1..text.len() - 1];
            if let Some(r) = parse_register(reg_str) {
                let offset = if offset_str.is_empty() {
                    0
                } else {
                    parse_number(offset_str)?
                };
                return Ok(MspOperand::Indexed(offset, r));
            }
        }
    }

    // Plain register
    if let Some(r) = parse_register(text) {
        return Ok(MspOperand::Register(r));
    }

    // Symbol/label
    Ok(MspOperand::Symbol(text.to_string()))
}

/// Parse an MSP430 register name, returning register number
fn parse_register(text: &str) -> Option<u8> {
    let text = text.trim().to_lowercase();
    match text.as_str() {
        "r0" | "pc" => Some(0),
        "r1" | "sp" => Some(1),
        "r2" | "sr" => Some(2),
        "r3" | "cg" => Some(3),
        "r4" => Some(4),
        "r5" => Some(5),
        "r6" => Some(6),
        "r7" => Some(7),
        "r8" => Some(8),
        "r9" => Some(9),
        "r10" => Some(10),
        "r11" => Some(11),
        "r12" => Some(12),
        "r13" => Some(13),
        "r14" => Some(14),
        "r15" => Some(15),
        _ => None,
    }
}

/// Parse a numeric literal (decimal, hex, negative)
fn parse_number(text: &str) -> Result<i32> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Ok(i32::from_str_radix(hex, 16)?)
    } else {
        Ok(text.parse::<i32>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_add() {
        let msp430 = r#"
	.section	.text.add,"ax",@progbits
	.globl	add
	.p2align	1
add:
	add	r13, r12
	ret
"#;
        let result = translate_msp430(msp430, "add").unwrap();
        assert!(result.contains("add     r0, r1"));
        // ret = pop r2 + jmp (r2)
        assert!(result.contains("pop     r2"));
        assert!(result.contains("jmp     (r2)"));
    }

    #[test]
    fn test_bitmask() {
        let msp430 = r#"
	.section	.text.bitmask,"ax",@progbits
	.globl	bitmask
bitmask:
	and	r13, r12
	ret
"#;
        let result = translate_msp430(msp430, "bitmask").unwrap();
        assert!(result.contains("and     r0, r1"));
    }

    #[test]
    fn test_mov_immediate() {
        let msp430 = r#"
	.section	.text.test,"ax",@progbits
test:
	mov	#1000, r12
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        assert!(result.contains("la      r0, 0x0003E8"));
    }

    #[test]
    fn test_compare_branch() {
        let msp430 = r#"
	.section	.text.compare_branch,"ax",@progbits
compare_branch:
	cmp	r13, r12
	jlo	.LBB4_2
	mov	r13, r12
.LBB4_2:
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        assert!(result.contains("clu     r0, r1"));
        assert!(result.contains("brt     .LBB4_2"));
    }

    #[test]
    fn test_blink_loop() {
        let msp430 = r#"
	.section	.text.blink_loop,"ax",@progbits
blink_loop:
.LBB2_1:
	mov	#-256, r12
	mov	#1, r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
	mov	#-256, r12
	clr	r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
	jmp	.LBB2_1
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        assert!(result.contains("bra     .LBB2_1"));
        // Should have stack-based calls (push return addr, jmp)
        assert!(result.contains("la      r2, mmio_write"));
        assert!(result.contains("jmp     (r2)"));
        // I/O address should be mapped: -256 (0xFF00) -> 0xFF0000
        assert!(result.contains("la      r0, 0xFF0000"));
    }

    #[test]
    fn test_button_echo() {
        let msp430 = r#"
	.section	.text.button_echo,"ax",@progbits
button_echo:
.LBB3_1:
	mov	#-256, r12
	call	#mmio_read
	mov	r12, r13
	and	#1, r13
	mov	#-256, r12
	call	#mmio_write
	jmp	.LBB3_1
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // Should translate the AND #1 pattern
        assert!(result.contains("and"));
    }

    #[test]
    fn test_spill_countdown() {
        // MSP430 demo_countdown uses r10 (callee-saved, spilled in COR24)
        let msp430 = r#"
	.section	.text.demo_countdown,"ax",@progbits
demo_countdown:
	push	r10
	mov	#10, r10
.LBB5_1:
	mov	#-256, r12
	mov	r10, r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
	add	#-1, r10
	tst	r10
	jne	.LBB5_1
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // push r10: load spill_10 into r1 (avoids clobbering r0), push r1
        assert!(result.contains("lw      r1, 18(fp)"));
        assert!(result.contains("push    r1"));
        // mov #10, r10: push/pop r0 to preserve it, load 10 into r0, store to spill slot
        assert!(result.contains("lc      r0, 10"));
        assert!(result.contains("sw      r0, 18(fp)"));
        // tst r10: push/pop r0 to preserve it, load from spill, ceq with z
        assert!(result.contains("ceq     r0, z"));
    }

    #[test]
    fn test_spill_fibonacci() {
        // MSP430 fibonacci uses r11, r14, r15 (r11 and r15 are spilled)
        let msp430 = r#"
	.section	.text.fibonacci,"ax",@progbits
fibonacci:
	cmp	#2, r12
	jhs	.LBB8_2
	mov	r12, r13
	jmp	.LBB8_4
.LBB8_2:
	mov	#2, r14
	clr	r15
	mov	#1, r13
.LBB8_3:
	mov	r13, r11
	mov	r15, r13
	add	r11, r13
	inc	r14
	cmp	r14, r12
	mov	r11, r15
	jhs	.LBB8_3
.LBB8_4:
	mov	r13, r12
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // r15 -> spill_15 (offset 24), r11 -> spill_11 (offset 21)
        // mov r13, r11: store r1 to spill_11 slot
        assert!(result.contains("sw      r1, 21(fp)"));
        // mov r15, r13: load spill_15 into r1
        assert!(result.contains("lw      r1, 24(fp)"));
        // mov r11, r15: both spilled, should use r0 as intermediary
        assert!(result.contains("lw      r0, 21(fp)"));
        // ret = pop r2, jmp (r2)
        assert!(result.contains("pop     r2"));
    }

    #[test]
    fn test_memory_ops() {
        let msp430 = r#"
	.section	.text.delay,"ax",@progbits
delay:
	sub	#2, r1
	tst	r12
	jeq	.LBB5_3
	add	#-1, r12
.LBB5_2:
	mov	r12, 0(r1)
	add	#-1, r12
	cmp	#-1, r12
	jne	.LBB5_2
.LBB5_3:
	add	#2, r1
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // Should have scaled stack adjustment (2 MSP430 bytes → 3 COR24 bytes)
        assert!(result.contains("sub     sp, 3"));
        // Should have store to stack (via temp reg since COR24 can't use sp as base)
        assert!(result.contains("mov     r2, sp"));
        assert!(result.contains("sw      r0, 0(r2)"));
    }

    // ============================================================
    // Entry point and pipeline tests
    // ============================================================

    /// Helper: MSP430 assembly for a multi-function file with .globl directives,
    /// mimicking real rustc output (panic handler first, then functions alphabetically)
    fn multi_function_msp430() -> &'static str {
        r#"
	.section	.text._RNvCs_panic,"ax",@progbits
	.globl	_RNvCs_panic
_RNvCs_panic:
.LBB0_1:
	jmp	.LBB0_1

	.section	.text.delay,"ax",@progbits
	.globl	delay
delay:
	sub	#2, r1
	tst	r12
	jeq	.LBB1_2
.LBB1_2:
	add	#2, r1
	ret

	.section	.text.demo_blinky,"ax",@progbits
	.globl	demo_blinky
demo_blinky:
	mov	#-256, r12
	mov	#1, r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
.LBB3_1:
	jmp	.LBB3_1

	.section	.text.mmio_write,"ax",@progbits
	.globl	mmio_write
mmio_write:
	mov	r13, 0(r12)
	ret

	.section	.text.start,"ax",@progbits
	.globl	start
start:
	call	#demo_blinky
"#
    }

    #[test]
    fn test_start_convention_produces_prologue() {
        // Positive: fixture has `start` label, default entry produces correct prologue
        let result = translate_msp430(multi_function_msp430(), "start").unwrap();
        assert!(result.starts_with("; COR24 Assembly"));
        assert!(result.contains("mov     fp, sp"), "prologue must init fp");
        assert!(result.contains("la      r0, start"));
        assert!(result.contains("jmp     (r0)"));
        // Prologue must come before any function code
        let bra_pos = result.find("la      r0, start").unwrap();
        let first_func = result.find("_RNvCs_panic:").unwrap();
        assert!(bra_pos < first_func, "bra prologue must precede first function");
    }

    #[test]
    fn test_explicit_entry_overrides_start() {
        // Positive: explicit --entry flag overrides default "start"
        let result = translate_msp430(multi_function_msp430(), "demo_blinky").unwrap();
        assert!(result.contains("la      r0, demo_blinky"));
        assert!(!result.contains("la      r0, start"));
    }

    #[test]
    fn test_missing_start_with_globl_fails() {
        // Negative: .globl present but no `start` label → error
        let msp430 = r#"
	.section	.text.helper,"ax",@progbits
	.globl	helper
helper:
	ret

	.section	.text.main,"ax",@progbits
	.globl	main
main:
	call	#helper
	ret
"#;
        let err = translate_msp430(msp430, "start").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("entry point 'start' not found"),
            "Expected 'not found' error, got: {}", msg);
    }

    #[test]
    fn test_nonexistent_entry_fails() {
        // Negative: explicit --entry with non-existent label fails
        let err = translate_msp430(multi_function_msp430(), "nonexistent").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("entry point 'nonexistent' not found"),
            "Expected 'not found' error, got: {}", msg);
        assert!(msg.contains("start"),
            "Error should list available labels, got: {}", msg);
    }

    #[test]
    fn test_entry_typo_fails() {
        // Negative: common typo in entry point name
        let err = translate_msp430(multi_function_msp430(), "strt").unwrap_err();
        assert!(err.to_string().contains("entry point 'strt' not found"));
    }

    #[test]
    fn test_spill_add_no_clobber() {
        // add r12, r10: r12->r0 (src), r10->spill_10 (dst)
        // Working register for spill load must NOT be r0 (would clobber live r12 value)
        // Must push/pop working register to preserve other live GP values
        let msp430 = r#"
	.section	.text.test,"ax",@progbits
test:
	add	r12, r10
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // Should push r1 (working reg), load spill, operate, store, pop r1
        assert!(result.contains("push    r1"), "should push working reg. Got:\n{}", result);
        assert!(result.contains("lw      r1, 18(fp)"), "spill should load into r1. Got:\n{}", result);
        assert!(result.contains("add     r1, r0"), "add should use r1 (spill) and r0 (r12). Got:\n{}", result);
        assert!(result.contains("sw      r1, 18(fp)"), "result should be stored back. Got:\n{}", result);
        assert!(result.contains("pop     r1"), "should pop working reg. Got:\n{}", result);
    }

    #[test]
    fn test_spill_add_src_r13() {
        // add r13, r10: r13->r1 (src), r10->spill_10 (dst)
        // Working register should be r0 (since r1 is the source)
        let msp430 = r#"
	.section	.text.test,"ax",@progbits
test:
	add	r13, r10
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        assert!(result.contains("push    r0"), "should push working reg. Got:\n{}", result);
        assert!(result.contains("lw      r0, 18(fp)"), "spill should load into r0. Got:\n{}", result);
        assert!(result.contains("add     r0, r1"), "add should use r0 (spill) and r1 (r13). Got:\n{}", result);
    }

    #[test]
    fn test_tail_call_optimization() {
        let msp430 = r#"
	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
uart_putc:
	mov	r12, r13
	mov	#-255, r12
	call	#mmio_write
	ret
"#;
        let result = translate_msp430(msp430, "uart_putc").unwrap();
        // Should be a tail call: la + jmp, no push/pop
        assert!(result.contains("; tail call mmio_write"), "Should have tail call comment. Got:\n{}", result);
        assert!(result.contains("la      r2, mmio_write"), "Should jump to target. Got:\n{}", result);
        assert!(!result.contains("push    r2"), "tail call should not push return address. Got:\n{}", result);
        // Should NOT have pop r2 / jmp (r2) for ret (it was consumed by tail call)
        assert!(!result.contains("pop     r2"), "tail call should not have ret sequence. Got:\n{}", result);
    }

    #[test]
    fn test_non_tail_call_preserved() {
        // call followed by more instructions (not ret) should NOT be optimized
        let msp430 = r#"
	.section	.text.test,"ax",@progbits
test:
	call	#helper
	mov	r12, r13
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        // Should be a normal call with push/pop
        assert!(result.contains("push    r2"), "non-tail call should push return addr. Got:\n{}", result);
        assert!(result.contains(".Lret_"), "non-tail call should have return label. Got:\n{}", result);
    }

    #[test]
    fn test_no_globl_no_prologue() {
        // No .globl directives = no prologue (legacy single-function case)
        let msp430 = r#"
	.section	.text.add,"ax",@progbits
add:
	add	r13, r12
	ret
"#;
        let result = translate_msp430(msp430, "start").unwrap();
        assert!(!result.contains("Reset vector"), "Should not emit prologue without .globl");
        assert!(result.contains("add     r0, r1"));
    }

    #[test]
    fn test_prologue_assembles_correctly() {
        // End-to-end: translated COR24 with prologue should assemble without errors
        let cor24 = translate_msp430(multi_function_msp430(), "start").unwrap();

        let mut assembler = cor24_emulator::assembler::Assembler::new();
        let result = assembler.assemble(&cor24);
        assert!(result.errors.is_empty(),
            "Assembly errors: {:?}", result.errors);
        assert!(result.bytes.len() > 4, "Binary should be > 4 bytes");
    }

    #[test]
    fn test_prologue_jumps_to_correct_address() {
        // End-to-end: load into CPU, execute prologue, verify PC lands at start
        let cor24 = translate_msp430(multi_function_msp430(), "start").unwrap();

        let mut assembler = cor24_emulator::assembler::Assembler::new();
        let result = assembler.assemble(&cor24);
        assert!(result.errors.is_empty(), "Assembly errors: {:?}", result.errors);

        // Find the address of start label
        let start_addr = result.lines.iter()
            .find(|l| l.source.trim() == "start:")
            .map(|l| l.address)
            .expect("start label should exist in assembled output");
        assert!(start_addr > 0, "start should not be at address 0");

        // Load and execute three instructions (mov fp,sp + la r0, start + jmp (r0))
        let mut cpu = cor24_emulator::cpu::state::CpuState::new();
        for line in &result.lines {
            for (i, &b) in line.bytes.iter().enumerate() {
                cpu.write_byte(line.address + i as u32, b);
            }
        }
        cpu.pc = 0;
        let executor = cor24_emulator::cpu::executor::Executor::new();
        executor.step(&mut cpu); // mov fp, sp
        executor.step(&mut cpu); // la r0, start
        executor.step(&mut cpu); // jmp (r0)

        assert_eq!(cpu.pc, start_addr,
            "After executing prologue, PC should be at start (0x{:06X}), got 0x{:06X}",
            start_addr, cpu.pc);
    }

    /// Echo v2: Rust logic + asm!() interrupt plumbing.
    /// Translates, assembles, runs, sends a/b/c → verifies A/B/C, sends ! → verifies halt.
    #[test]
    fn test_echo_v2_rust_logic() {
        use cor24_emulator::assembler::Assembler;
        use cor24_emulator::cpu::state::CpuState;
        use cor24_emulator::cpu::executor::Executor;

        let msp430_src = include_str!("../demos/demo_echo_v2/demo_echo_v2.msp430.s");
        let cor24 = translate_msp430(msp430_src, "start").unwrap();

        let mut asm = Assembler::new();
        let result = asm.assemble(&cor24);
        assert!(result.errors.is_empty(), "Assembly errors: {:?}\nCOR24:\n{}", result.errors, cor24);

        let mut cpu = CpuState::new();
        for line in &result.lines {
            for (i, &b) in line.bytes.iter().enumerate() {
                cpu.write_byte(line.address + i as u32, b);
            }
        }
        cpu.pc = 0;
        let executor = Executor::new();

        // Run to prompt
        executor.run(&mut cpu, 10_000);
        assert_eq!(cpu.io.uart_output, "?", "Prompt should appear");
        assert!(!cpu.halted, "Should not be halted yet");

        // Send 'a' → 'A'
        cpu.uart_send_rx(b'a');
        executor.run(&mut cpu, 10_000);
        assert_eq!(cpu.io.uart_output, "?A", "'a' -> 'A'");

        // Send 'b' → 'B'
        cpu.uart_send_rx(b'b');
        executor.run(&mut cpu, 10_000);
        assert_eq!(cpu.io.uart_output, "?AB", "'b' -> 'B'");

        // Send 'c' → 'C'
        cpu.uart_send_rx(b'c');
        executor.run(&mut cpu, 10_000);
        assert_eq!(cpu.io.uart_output, "?ABC", "'c' -> 'C'");

        // Send '3' → '3' (digit, as-is)
        cpu.uart_send_rx(b'3');
        executor.run(&mut cpu, 10_000);
        assert_eq!(cpu.io.uart_output, "?ABC3", "'3' -> '3'");

        // Send '!' → halt
        cpu.uart_send_rx(b'!');
        executor.run(&mut cpu, 10_000);
        assert!(cpu.halted, "Should halt on '!'");

        // Dump state
        eprintln!("=== Echo v2 final state ===");
        eprintln!("PC: 0x{:06X}  Halted: {}", cpu.pc, cpu.halted);
        eprintln!("Registers: r0=0x{:06X} r1=0x{:06X} r2=0x{:06X}",
            cpu.registers[0], cpu.registers[1], cpu.registers[2]);
        eprintln!("  fp=0x{:06X} sp=0x{:06X}", cpu.registers[3], cpu.registers[4]);
        eprintln!("UART output: {:?}", cpu.io.uart_output);
    }
}
