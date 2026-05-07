//! EmulatorCore: shared emulator controller for CLI and Web UI
//!
//! Provides run_batch(), pause/resume, breakpoints, and I/O accessors.
//! Both the CLI debugger and Yew/WASM UI call this directly (no IPC).

use crate::cpu::executor::{ExecuteResult, Executor};
use crate::cpu::instruction::{DecodedInstruction, Opcode};
use crate::cpu::state::{CpuState, DecodeRom};
use crate::loader;

/// Why a batch stopped
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Ran the requested number of instructions
    CycleLimit,
    /// Hit a breakpoint at this address
    Breakpoint(u32),
    /// CPU halted (executed halt sentinel)
    Halted,
    /// Invalid instruction encountered
    InvalidInstruction(u8),
    /// Emulator is paused (pause was requested)
    Paused,
    /// Stack overflow: SP went below stack base
    StackOverflow(u32),
    /// Stack underflow: SP went above stack top (initial SP)
    StackUnderflow(u32),
}

/// Result of a run_batch call
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of instructions executed in this batch
    pub instructions_run: u64,
    /// Why execution stopped
    pub reason: StopReason,
    /// Number of new UART output bytes since batch started
    pub uart_bytes_added: usize,
    /// Whether LED state changed during batch
    pub led_changed: bool,
}

/// Snapshot of CPU state for UI display
#[derive(Debug, Clone)]
pub struct CpuSnapshot {
    pub regs: [u32; 8],
    pub pc: u32,
    pub c: bool,
    pub halted: bool,
    pub intis: bool,
    pub cycles: u64,
    pub instructions: u64,
    pub led: u8,
    pub button: u8,
    pub uart_output_len: usize,
}

/// The shared emulator core
#[derive(Clone)]
pub struct EmulatorCore {
    cpu: CpuState,
    executor: Executor,
    decode_rom: DecodeRom,
    breakpoints: Vec<u32>,
    paused: bool,
    /// Highest address written during load (tracks end of code+data)
    program_end: u32,
    /// Valid stack region bounds. Both non-zero enables checking.
    /// SP below stack_base → StackOverflow, SP above stack_top → StackUnderflow.
    /// Set both to 0 to disable (e.g., custom runtimes with non-EBR stacks).
    stack_base: u32,
    stack_top: u32,
}

impl Default for EmulatorCore {
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorCore {
    pub fn new() -> Self {
        Self {
            cpu: CpuState::new(),
            executor: Executor::new(),
            decode_rom: DecodeRom::new(),
            breakpoints: Vec::new(),
            paused: true, // start paused
            program_end: 0,
            stack_base: crate::cpu::state::EBR_BASE,
            stack_top: crate::cpu::state::INITIAL_SP,
        }
    }

    // ===== Load =====

    /// Load an LGO file into memory, set entry point
    pub fn load_lgo(&mut self, content: &str, entry: Option<u32>) -> Result<usize, String> {
        self.cpu = CpuState::new();
        let result = loader::load_lgo(content, &mut self.cpu)?;
        let start = entry.or(result.start_addr).unwrap_or(0);
        self.cpu.pc = start;
        self.paused = true;
        self.program_end = result.highest_address;
        Ok(result.bytes_loaded)
    }

    /// Load assembled bytes into memory at given address
    pub fn load_program(&mut self, addr: u32, bytes: &[u8]) {
        self.cpu.load_program(addr, bytes);
        let end = addr + bytes.len() as u32;
        if end > self.program_end {
            self.program_end = end;
        }
    }

    /// Set the program extent (highest address used by program code)
    pub fn load_program_extent(&mut self, end: u32) {
        if end > self.program_end {
            self.program_end = end;
        }
    }

    /// Reset CPU (preserves memory)
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.paused = true;
    }

    /// Hard reset (clears memory too)
    pub fn hard_reset(&mut self) {
        self.cpu.hard_reset();
        self.paused = true;
    }

    // ===== Run control =====

    /// Run a batch of instructions. Returns what happened.
    pub fn run_batch(&mut self, max_instructions: u64) -> BatchResult {
        let uart_before = self.cpu.io.uart_output.len();
        let led_before = self.cpu.io.leds;
        let mut count = 0u64;

        if self.cpu.halted {
            return BatchResult {
                instructions_run: 0,
                reason: StopReason::Halted,
                uart_bytes_added: 0,
                led_changed: false,
            };
        }

        if self.paused {
            return BatchResult {
                instructions_run: 0,
                reason: StopReason::Paused,
                uart_bytes_added: 0,
                led_changed: false,
            };
        }

        let reason = loop {
            if count >= max_instructions {
                break StopReason::CycleLimit;
            }

            // Check breakpoints (skip on first instruction to allow continuing past one)
            if count > 0 && self.breakpoints.contains(&self.cpu.pc) {
                self.paused = true;
                break StopReason::Breakpoint(self.cpu.pc);
            }

            match self.executor.step(&mut self.cpu) {
                ExecuteResult::Ok => {}
                ExecuteResult::Halted => {
                    break StopReason::Halted;
                }
                ExecuteResult::InvalidInstruction(byte) => {
                    self.cpu.halted = true;
                    break StopReason::InvalidInstruction(byte);
                }
                ExecuteResult::MemoryError(_) => {
                    self.cpu.halted = true;
                    break StopReason::Halted;
                }
            }
            count += 1;

            // Check for stack bounds: overflow (below base) or underflow (above top)
            if self.stack_base != 0 {
                let sp = self.cpu.get_reg(4);
                if sp < self.stack_base {
                    self.cpu.halted = true;
                    break StopReason::StackOverflow(sp);
                } else if sp > self.stack_top {
                    self.cpu.halted = true;
                    break StopReason::StackUnderflow(sp);
                }
            }
        };

        BatchResult {
            instructions_run: count,
            reason,
            uart_bytes_added: self.cpu.io.uart_output.len() - uart_before,
            led_changed: self.cpu.io.leds != led_before,
        }
    }

    /// Execute exactly one instruction
    pub fn step(&mut self) -> BatchResult {
        self.paused = false;
        let result = self.run_batch(1);
        self.paused = true;
        result
    }

    /// Step over: execute one instruction, but if it's a jal (call),
    /// run until return (matching stack depth)
    pub fn step_over(&mut self) -> BatchResult {
        let inst_byte = self.cpu.read_byte(self.cpu.pc);
        let decoded = self.decode_rom.decode(inst_byte);
        let is_call = if decoded != 0xFFF {
            let inst = DecodedInstruction::from_decoded(decoded);
            inst.opcode == Opcode::Jal
        } else {
            false
        };

        if !is_call {
            return self.step();
        }

        // It's a call — set a temporary breakpoint after the instruction
        let return_addr = self.cpu.pc + 1; // jal is 1 byte
        let had_bp = self.breakpoints.contains(&return_addr);
        if !had_bp {
            self.breakpoints.push(return_addr);
        }
        self.paused = false;
        let result = self.run_batch(10_000_000);
        if !had_bp {
            self.breakpoints.retain(|&a| a != return_addr);
        }
        self.paused = true;
        result
    }

    /// Start running (unpause)
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Pause execution
    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.halted
    }

    pub fn is_interrupt_in_service(&self) -> bool {
        self.cpu.intis
    }

    pub fn is_running(&self) -> bool {
        !self.paused && !self.cpu.halted
    }

    // ===== Breakpoints =====

    pub fn add_breakpoint(&mut self, addr: u32) -> bool {
        if self.breakpoints.contains(&addr) {
            false
        } else {
            self.breakpoints.push(addr);
            true
        }
    }

    pub fn remove_breakpoint(&mut self, addr: u32) -> bool {
        let len = self.breakpoints.len();
        self.breakpoints.retain(|&a| a != addr);
        self.breakpoints.len() < len
    }

    pub fn remove_breakpoint_by_index(&mut self, index: usize) -> Option<u32> {
        if index < self.breakpoints.len() {
            Some(self.breakpoints.remove(index))
        } else {
            None
        }
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn breakpoints(&self) -> &[u32] {
        &self.breakpoints
    }

    pub fn has_breakpoint(&self, addr: u32) -> bool {
        self.breakpoints.contains(&addr)
    }

    // ===== State queries =====

    pub fn snapshot(&self) -> CpuSnapshot {
        CpuSnapshot {
            regs: [
                self.cpu.get_reg(0),
                self.cpu.get_reg(1),
                self.cpu.get_reg(2),
                self.cpu.get_reg(3),
                self.cpu.get_reg(4),
                self.cpu.get_reg(5),
                self.cpu.get_reg(6),
                self.cpu.get_reg(7),
            ],
            pc: self.cpu.pc,
            c: self.cpu.c,
            halted: self.cpu.halted,
            intis: self.cpu.intis,
            cycles: self.cpu.cycles,
            instructions: self.cpu.instructions,
            led: self.cpu.io.leds,
            button: self.cpu.io.switches,
            uart_output_len: self.cpu.io.uart_output.len(),
        }
    }

    pub fn pc(&self) -> u32 {
        self.cpu.pc
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.cpu.pc = pc;
    }

    pub fn get_reg(&self, reg: u8) -> u32 {
        self.cpu.get_reg(reg)
    }

    pub fn set_reg(&mut self, reg: u8, value: u32) {
        self.cpu.set_reg(reg, value);
    }

    /// Set valid stack bounds. SP below `base` → overflow, SP above `top` → underflow.
    /// Set both to 0 to disable stack checking (e.g., custom runtimes).
    pub fn set_stack_bounds(&mut self, base: u32, top: u32) {
        self.stack_base = base;
        self.stack_top = top;
    }

    pub fn condition_flag(&self) -> bool {
        self.cpu.c
    }

    pub fn cycles(&self) -> u64 {
        self.cpu.cycles
    }

    /// Highest address written during load (end of code+data region)
    pub fn program_end(&self) -> u32 {
        self.program_end
    }

    pub fn instructions_count(&self) -> u64 {
        self.cpu.instructions
    }

    // ===== Memory access =====

    pub fn read_byte(&self, addr: u32) -> u8 {
        self.cpu.read_byte(addr)
    }

    pub fn write_byte(&mut self, addr: u32, value: u8) {
        self.cpu.write_byte(addr, value);
    }

    pub fn read_word(&self, addr: u32) -> u32 {
        self.cpu.read_word(addr)
    }

    pub fn read_memory(&self, addr: u32, len: u32) -> Vec<u8> {
        (0..len).map(|i| self.cpu.read_byte(addr + i)).collect()
    }

    /// Direct read-only access to SRAM backing memory (0x000000-0x0FFFFF)
    pub fn sram(&self) -> &[u8] {
        &self.cpu.memory
    }

    /// Direct read-only access to EBR backing memory (0xFEE000-0xFEFFFF)
    pub fn ebr(&self) -> &[u8] {
        &self.cpu.ebr
    }

    // ===== I/O: LED and Button =====

    /// Get raw LED register value (active-low: 0=ON, 1=OFF)
    pub fn get_led(&self) -> u8 {
        self.cpu.io.leds
    }

    /// Check if LED D2 is visually on (active-low: register bit 0 == 0 means ON)
    pub fn is_led_on(&self) -> bool {
        (self.cpu.io.leds & 1) == 0
    }

    pub fn is_button_pressed(&self) -> bool {
        // Button S2: bit 0, normally high, low = pressed
        self.cpu.io.switches & 1 == 0
    }

    pub fn get_button(&self) -> bool {
        self.is_button_pressed()
    }

    pub fn set_button_pressed(&mut self, pressed: bool) {
        if pressed {
            self.cpu.io.switches &= !1; // low = pressed
        } else {
            self.cpu.io.switches |= 1; // high = released
        }
    }

    // ===== I/O: UART =====

    pub fn get_uart_output(&self) -> &str {
        &self.cpu.io.uart_output
    }

    pub fn clear_uart_output(&mut self) {
        self.cpu.io.uart_output.clear();
    }

    pub fn uart_log(&self) -> &crate::cpu::state::UartLog {
        &self.cpu.io.uart_log
    }

    pub fn format_uart_log(&self) -> String {
        self.cpu.io.uart_log.format()
    }

    pub fn send_uart_byte(&mut self, byte: u8) {
        self.cpu.uart_send_rx(byte);
    }

    // ===== Trace =====

    /// Get the instruction trace buffer
    pub fn trace(&self) -> &crate::cpu::state::TraceBuffer {
        &self.cpu.trace
    }

    /// Set UART TX busy duration (cycles after each write before TX is ready again).
    /// Set to 0 for instant TX (no busy wait needed).
    pub fn set_uart_tx_busy_cycles(&mut self, cycles: u8) {
        self.cpu.io.uart_tx_busy_cycles = cycles;
    }

    /// Set UART TX never-ready mode (for testing poll-before-write discipline)
    pub fn set_uart_never_ready(&mut self, never_ready: bool) {
        self.cpu.io.uart_never_ready = never_ready;
        if never_ready {
            self.cpu.io.uart_tx_busy = true;
        }
    }

    // ===== Disassembly =====

    /// Disassemble one instruction at the given address.
    /// Returns (mnemonic_string, instruction_size_in_bytes).
    pub fn disassemble_at(&self, addr: u32) -> (String, u32) {
        use crate::cpu::instruction::{InstructionFormat, reg_name};

        let inst_byte = self.cpu.read_byte(addr);
        let decoded = self.decode_rom.decode(inst_byte);
        if decoded == 0xFFF {
            return (format!(".byte 0x{:02X}", inst_byte), 1);
        }

        let inst = DecodedInstruction::from_decoded(decoded);
        let format = inst.opcode.format();
        let size = match format {
            InstructionFormat::SingleByte => 1,
            InstructionFormat::TwoBytes => 2,
            InstructionFormat::FourBytes => 4,
        };

        let text = match format {
            InstructionFormat::SingleByte => match inst.opcode {
                Opcode::Pop => format!("pop  {}", reg_name(inst.ra)),
                Opcode::Push => format!("push {}", reg_name(inst.ra)),
                Opcode::Jmp => format!("jmp  ({})", reg_name(inst.ra)),
                Opcode::Jal => {
                    format!("jal  {},({})", reg_name(inst.ra), reg_name(inst.rb))
                }
                Opcode::Mov if inst.rb == 5 => {
                    format!("mov  {},c", reg_name(inst.ra))
                }
                _ => {
                    format!(
                        "{:4} {},{}",
                        inst.opcode.mnemonic(),
                        reg_name(inst.ra),
                        reg_name(inst.rb)
                    )
                }
            },
            InstructionFormat::TwoBytes => {
                let imm = self.cpu.read_byte(addr + 1);
                let signed = imm as i8;
                match inst.opcode {
                    Opcode::Bra | Opcode::Brf | Opcode::Brt => {
                        let branch_base = addr.wrapping_add(4);
                        let target = CpuState::mask_24(
                            branch_base.wrapping_add(CpuState::sign_extend_8(imm)),
                        );
                        format!("{:4} 0x{:06X}", inst.opcode.mnemonic(), target)
                    }
                    Opcode::Lc | Opcode::Lcu | Opcode::AddImm => {
                        format!(
                            "{:4} {},{}",
                            inst.opcode.mnemonic(),
                            reg_name(inst.ra),
                            signed
                        )
                    }
                    _ => {
                        // Lb, Lbu, Lw, Sb, Sw with offset
                        if signed == 0 {
                            format!(
                                "{:4} {},({})",
                                inst.opcode.mnemonic(),
                                reg_name(inst.ra),
                                reg_name(inst.rb)
                            )
                        } else {
                            format!(
                                "{:4} {},{}({})",
                                inst.opcode.mnemonic(),
                                reg_name(inst.ra),
                                signed,
                                reg_name(inst.rb)
                            )
                        }
                    }
                }
            }
            InstructionFormat::FourBytes => {
                let b0 = self.cpu.read_byte(addr + 1) as u32;
                let b1 = self.cpu.read_byte(addr + 2) as u32;
                let b2 = self.cpu.read_byte(addr + 3) as u32;
                let imm24 = b0 | (b1 << 8) | (b2 << 16);
                format!(
                    "{:4} {},0x{:06X}",
                    inst.opcode.mnemonic(),
                    reg_name(inst.ra),
                    imm24
                )
            }
        };

        (text, size)
    }

    /// Disassemble N instructions starting at addr
    pub fn disassemble(&self, addr: u32, count: usize) -> Vec<(u32, String, u32)> {
        let mut result = Vec::with_capacity(count);
        let mut pc = addr;
        for _ in 0..count {
            let (text, size) = self.disassemble_at(pc);
            result.push((pc, text, size));
            pc = CpuState::mask_24(pc + size);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble `source` via the standalone `cor24-asm` binary and
    /// return the `.lgo` text. Used by tests below that previously
    /// called the in-tree `Assembler` directly. Requires `cor24-asm`
    /// on PATH (saga decision: the in-tree assembler is going away;
    /// production toolchain is the canonical assembler).
    fn asm_to_lgo(source: &str) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("cor24-asm")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("cor24-asm not on PATH; required by emulator tests");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let output = child
            .wait_with_output()
            .expect("cor24-asm wait_with_output failed");
        if !output.status.success() {
            panic!(
                "cor24-asm failed: status={:?}\nstderr=\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        String::from_utf8(output.stdout).expect("cor24-asm output not UTF-8")
    }

    #[test]
    fn test_new_starts_paused() {
        let emu = EmulatorCore::new();
        assert!(emu.is_paused());
        assert!(!emu.is_halted());
    }

    #[test]
    fn test_step_executes_one() {
        let mut emu = EmulatorCore::new();
        // lc r0,42 at address 0
        emu.write_byte(0, 0x44); // lc r0
        emu.write_byte(1, 42);
        let result = emu.step();
        assert_eq!(result.instructions_run, 1);
        assert_eq!(emu.get_reg(0), 42);
        assert!(emu.is_paused()); // step re-pauses
    }

    #[test]
    fn test_run_batch_paused_does_nothing() {
        let mut emu = EmulatorCore::new();
        let result = emu.run_batch(1000);
        assert_eq!(result.instructions_run, 0);
        assert_eq!(result.reason, StopReason::Paused);
    }

    #[test]
    fn test_run_batch_with_resume() {
        let mut emu = EmulatorCore::new();
        // add r0,r0 at address 0 (valid but will hit halt at pc=0 on second iteration)
        emu.write_byte(0, 0x00); // add r0,r0
        emu.resume();
        let result = emu.run_batch(10);
        // Should halt since byte 0x00 at address 0 is the halt sentinel
        assert_eq!(result.reason, StopReason::Halted);
    }

    #[test]
    fn test_breakpoint() {
        let mut emu = EmulatorCore::new();
        // lc r0,1 ; lc r0,2 ; lc r0,3
        emu.write_byte(0, 0x44);
        emu.write_byte(1, 1);
        emu.write_byte(2, 0x44);
        emu.write_byte(3, 2);
        emu.write_byte(4, 0x44);
        emu.write_byte(5, 3);
        emu.add_breakpoint(4);
        emu.resume();
        let result = emu.run_batch(100);
        assert_eq!(result.reason, StopReason::Breakpoint(4));
        assert_eq!(emu.pc(), 4);
        assert_eq!(emu.get_reg(0), 2); // executed lc r0,1 and lc r0,2
    }

    #[test]
    fn test_button_toggle() {
        let mut emu = EmulatorCore::new();
        assert!(!emu.is_button_pressed()); // not pressed (high)
        emu.set_button_pressed(true);
        assert!(emu.is_button_pressed()); // pressed (low)
        emu.set_button_pressed(false);
        assert!(!emu.is_button_pressed()); // released
    }

    #[test]
    fn test_uart_io() {
        let mut emu = EmulatorCore::new();
        // Write 'A' to UART via store byte
        emu.write_byte(0xFF0100, b'A');
        assert_eq!(emu.get_uart_output(), "A");

        emu.send_uart_byte(b'Z');
        assert_eq!(emu.read_byte(0xFF0100), b'Z');
    }

    #[test]
    fn test_led_change_detected() {
        let mut emu = EmulatorCore::new();
        // la r0, 0xFF0000 (LED addr); lc r1, 0; sb r1, 0(r0)
        // Write 0 to LED = ON (active-low)
        emu.write_byte(0, 0x29); // la r0
        emu.write_byte(1, 0x00);
        emu.write_byte(2, 0x00);
        emu.write_byte(3, 0xFF); // 0xFF0000
        emu.write_byte(4, 0x45); // lc r1
        emu.write_byte(5, 0x00); // value 0 (LED ON, active-low)
        emu.write_byte(6, 0x84); // sb r1,(r0)
        emu.write_byte(7, 0x00); // offset 0

        emu.resume();
        let result = emu.run_batch(3);
        assert!(result.led_changed);
        assert_eq!(emu.get_led(), 0);
        assert!(emu.is_led_on());
    }

    #[test]
    fn test_load_sieve_and_run() {
        let lgo = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/research/asld24/sieve.lgo"
        ))
        .expect("sieve.lgo must exist");

        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, Some(0x93)).unwrap();
        emu.resume();
        let result = emu.run_batch(1_000_000);
        assert!(
            emu.get_uart_output().starts_with("1000 iterations"),
            "Sieve should start printing, got: {:?}",
            emu.get_uart_output()
        );
        assert!(result.uart_bytes_added > 0);
    }

    #[test]
    fn test_disassemble() {
        let mut emu = EmulatorCore::new();
        emu.write_byte(0, 0x80); // push fp
        emu.write_byte(1, 0x65); // mov fp,sp
        emu.write_byte(2, 0x44); // lc r0
        emu.write_byte(3, 42); // value 42

        let lines = emu.disassemble(0, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, 0); // addr
        assert!(lines[0].1.contains("push"));
        assert_eq!(lines[1].0, 1);
        assert!(lines[1].1.contains("mov"));
        assert_eq!(lines[2].0, 2);
        assert!(lines[2].1.contains("lc"));
    }

    /// Reproduce web UI pattern: single-step loop with UART send between ticks
    #[test]
    fn test_echo_via_single_step_loop() {
        let source = include_str!("examples/assembler/echo.s");
        let lgo = asm_to_lgo(source);

        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_pc(0);

        fn run_tick(emu: &mut EmulatorCore, steps: u32) {
            for _ in 0..steps {
                if emu.is_halted() {
                    break;
                }
                let r = emu.step();
                if matches!(
                    r.reason,
                    StopReason::Halted | StopReason::InvalidInstruction(_)
                ) {
                    break;
                }
            }
        }

        // Init — prompt should appear
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?", "Prompt should appear");
        assert!(!emu.is_halted(), "Should not be halted");

        // Send 'a' (lowercase) → echoes 'A' (uppercase)
        emu.send_uart_byte(b'a');
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?A", "'a' -> 'A'");

        // Send 'B' (uppercase) → echoes 'B' as-is
        emu.send_uart_byte(b'B');
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?AB", "'B' -> 'B'");

        // Send '1' (digit) → echoes '1' as-is
        emu.send_uart_byte(b'1');
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?AB1", "'1' -> '1'");

        // Send '!' → halts
        emu.send_uart_byte(b'!');
        run_tick(&mut emu, 500);
        assert!(emu.is_halted(), "Should halt on '!'");
    }

    /// Test echo using the embedded source (same path the deprecated web UI used)
    #[test]
    fn test_echo_via_challenge_source() {
        let source = include_str!("examples/assembler/echo.s");
        let lgo = asm_to_lgo(source);

        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_pc(0);

        fn run_tick(emu: &mut EmulatorCore, steps: u32) {
            for _ in 0..steps {
                if emu.is_halted() {
                    break;
                }
                let r = emu.step();
                if matches!(
                    r.reason,
                    StopReason::Halted | StopReason::InvalidInstruction(_)
                ) {
                    break;
                }
            }
        }

        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?", "Prompt");

        emu.send_uart_byte(b'c');
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?C", "'c' -> 'C'");

        emu.send_uart_byte(b'?');
        run_tick(&mut emu, 500);
        assert_eq!(emu.get_uart_output(), "?C?", "'?' -> '?' as-is");

        // '!' halts
        emu.send_uart_byte(b'!');
        run_tick(&mut emu, 500);
        assert!(emu.is_halted(), "Should halt on '!'");
    }

    /// Verify UART log captures a full echo session with coalesced output
    #[test]
    fn test_uart_log_echo_session() {
        let source = include_str!("examples/assembler/echo.s");
        let lgo = asm_to_lgo(source);

        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_pc(0);

        fn run_tick(emu: &mut EmulatorCore, steps: u32) {
            for _ in 0..steps {
                if emu.is_halted() {
                    break;
                }
                let r = emu.step();
                if matches!(
                    r.reason,
                    StopReason::Halted | StopReason::InvalidInstruction(_)
                ) {
                    break;
                }
            }
        }

        // Prompt "?"
        run_tick(&mut emu, 500);

        // Send a few characters
        emu.send_uart_byte(b'a');
        run_tick(&mut emu, 500);
        emu.send_uart_byte(b'b');
        run_tick(&mut emu, 500);
        emu.send_uart_byte(b'!');
        run_tick(&mut emu, 500);

        let log = emu.format_uart_log();

        // Log should contain prompt output, input chars, and echo output
        assert!(
            log.contains("OUT:"),
            "Log should have output entries:\n{}",
            log
        );
        assert!(
            log.contains("IN:"),
            "Log should have input entries:\n{}",
            log
        );

        // Verify the coalesced structure: prompt is one OUT group,
        // then alternating IN/OUT for each echoed character
        let lines: Vec<&str> = log.lines().collect();
        assert!(lines.len() >= 3, "Should have multiple log lines:\n{}", log);

        // First line should be the prompt output "?"
        assert!(
            lines[0].contains("OUT:") && lines[0].contains("?"),
            "First line should be prompt: {}",
            lines[0]
        );

        // Print for manual inspection during development
        eprintln!("--- UART Log ---\n{}", log);
    }

    #[test]
    fn test_stack_overflow_detected() {
        // Blind recursion with no self-check — SP walks below EBR base
        let source = "
            bra  recurse
        recurse:
            push r0
            bra  recurse
        ";
        let lgo = asm_to_lgo(source);
        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_pc(0);
        emu.resume();

        let batch = emu.run_batch(500_000);
        assert!(
            matches!(batch.reason, StopReason::StackOverflow(_)),
            "Should stop with StackOverflow, got {:?}",
            batch.reason
        );
        assert!(emu.is_halted());
        assert!(
            emu.get_reg(4) < crate::cpu::state::EBR_BASE,
            "SP 0x{:06X} should be below EBR base",
            emu.get_reg(4)
        );
    }

    #[test]
    fn test_stack_underflow_detected() {
        // Pop more than pushed — SP goes above initial SP
        let source = "
            pop r0
            pop r0
            bra halt
        halt:
            bra halt
        ";
        let lgo = asm_to_lgo(source);
        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_pc(0);
        emu.resume();

        let batch = emu.run_batch(100);
        assert!(
            matches!(batch.reason, StopReason::StackUnderflow(_)),
            "Should stop with StackUnderflow, got {:?}",
            batch.reason
        );
        assert!(emu.is_halted());
        assert!(
            emu.get_reg(4) > crate::cpu::state::INITIAL_SP,
            "SP 0x{:06X} should be above initial SP",
            emu.get_reg(4)
        );
    }

    #[test]
    fn test_stack_bounds_disabled() {
        // Same blind recursion, but with bounds checking disabled — should not trigger
        let source = "
            bra  recurse
        recurse:
            push r0
            bra  recurse
        ";
        let lgo = asm_to_lgo(source);
        let mut emu = EmulatorCore::new();
        emu.load_lgo(&lgo, None).expect("load_lgo");
        emu.set_stack_bounds(0, 0); // disable checking
        emu.set_pc(0);
        emu.resume();

        let batch = emu.run_batch(2000);
        assert_eq!(
            batch.reason,
            StopReason::CycleLimit,
            "Should hit cycle limit (no stack fault), got {:?}",
            batch.reason
        );
    }
}
