//! WASM bindings for the COR24 CPU simulator
//!
//! This module provides JavaScript-accessible interfaces to the CPU.
//! WasmCpu wraps EmulatorCore for consistent behavior with the CLI.

use crate::assembler::{Assembler, AssemblyResult};
use crate::challenge::get_challenges;
use crate::cpu::{CpuState, ExecuteResult, Executor};
use crate::emulator::EmulatorCore;
use wasm_bindgen::prelude::*;
use web_sys::console;

/// WASM-accessible CPU wrapper backed by EmulatorCore
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmCpu {
    emu: EmulatorCore,
    last_result: Option<AssemblyResult>,
}

impl Default for WasmCpu {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmCpu {
    /// Create a new CPU
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            emu: EmulatorCore::new(),
            last_result: None,
        }
    }

    /// Reset the CPU to initial state (preserves memory)
    pub fn reset(&mut self) {
        self.emu.reset();
    }

    /// Hard reset - clears memory too
    pub fn hard_reset(&mut self) {
        self.emu.hard_reset();
        self.last_result = None;
    }

    /// Assemble source code and load into memory
    pub fn assemble(&mut self, source: &str) -> Result<JsValue, JsValue> {
        let mut assembler = Assembler::new();
        let result = assembler.assemble(source);

        if result.errors.is_empty() {
            self.emu.hard_reset();
            // Load bytes at their correct addresses, tracking program extent
            let mut highest_addr: u32 = 0;
            for line in &result.lines {
                for (i, &b) in line.bytes.iter().enumerate() {
                    let addr = line.address + i as u32;
                    self.emu.write_byte(addr, b);
                    if addr + 1 > highest_addr {
                        highest_addr = addr + 1;
                    }
                }
            }
            // Use load_program with empty slice to set program_end correctly
            self.emu.load_program_extent(highest_addr);

            console::log_1(&JsValue::from_str(&format!(
                "Loaded {} bytes into memory",
                result.bytes.len()
            )));

            self.last_result = Some(result.clone());
            serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str(&result.errors.join("\n")))
        }
    }

    /// Get the assembled lines for display
    pub fn get_assembled_lines(&self) -> Vec<String> {
        if let Some(result) = &self.last_result {
            result
                .lines
                .iter()
                .map(|line| {
                    if line.bytes.is_empty() {
                        format!("       {}", line.source)
                    } else {
                        let bytes_str: String = line
                            .bytes
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        format!("{:04X}: {:12} {}", line.address, bytes_str, line.source)
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Execute one instruction
    pub fn step(&mut self) -> Result<bool, JsValue> {
        let result = self.emu.step();
        match result.reason {
            crate::emulator::StopReason::CycleLimit => Ok(true),
            crate::emulator::StopReason::Halted => Ok(false),
            crate::emulator::StopReason::InvalidInstruction(byte) => Err(JsValue::from_str(
                &format!("Invalid instruction: 0x{:02X}", byte),
            )),
            _ => Ok(true),
        }
    }

    /// Run a batch of instructions (for animated execution)
    pub fn run_batch(&mut self, max_instructions: u32) -> bool {
        self.emu.resume();
        let result = self.emu.run_batch(max_instructions as u64);
        !matches!(result.reason, crate::emulator::StopReason::Halted | crate::emulator::StopReason::InvalidInstruction(_))
    }

    /// Run until halt or error
    pub fn run(&mut self) -> Result<(), JsValue> {
        self.emu.resume();
        let result = self.emu.run_batch(100_000);
        match result.reason {
            crate::emulator::StopReason::CycleLimit
            | crate::emulator::StopReason::Halted
            | crate::emulator::StopReason::Paused
            | crate::emulator::StopReason::Breakpoint(_) => Ok(()),
            crate::emulator::StopReason::InvalidInstruction(byte) => Err(JsValue::from_str(
                &format!("Invalid instruction: 0x{:02X}", byte),
            )),
        }
    }

    /// Check if CPU is halted
    pub fn is_halted(&self) -> bool {
        self.emu.is_halted()
    }

    /// Get program counter
    pub fn pc(&self) -> u32 {
        self.emu.pc()
    }

    /// Get instruction count
    pub fn instruction_count(&self) -> u64 {
        self.emu.instructions_count()
    }

    /// Get condition flag
    pub fn get_c_flag(&self) -> bool {
        self.emu.condition_flag()
    }

    /// Read a register value
    pub fn read_register(&self, reg: u8) -> u32 {
        self.emu.get_reg(reg)
    }

    /// Get all register values as an array
    pub fn get_registers(&self) -> Vec<u32> {
        (0..8).map(|i| self.emu.get_reg(i)).collect()
    }

    /// Read memory byte
    pub fn read_memory(&self, addr: u32) -> u8 {
        self.emu.read_byte(addr)
    }

    /// Get memory slice as bytes
    pub fn get_memory_slice(&self, start: u32, len: u32) -> Vec<u8> {
        self.emu.read_memory(start, len)
    }

    // ===== I/O Peripheral Access =====

    /// Get LED state (8 bits)
    pub fn get_leds(&self) -> u8 {
        self.emu.get_led()
    }

    /// Get switch state (8 bits)
    pub fn get_switches(&self) -> u8 {
        if self.emu.get_button() { 0xFE } else { 0xFF }
    }

    /// Set switch state (simulates external switch input)
    pub fn set_switches(&mut self, value: u8) {
        self.emu.set_button_pressed(value & 1 == 1);
    }

    /// Toggle a specific switch bit
    pub fn toggle_switch(&mut self, bit: u8) {
        if bit == 0 {
            let pressed = self.emu.get_button();
            self.emu.set_button_pressed(!pressed);
        }
    }

    /// Get UART output buffer
    pub fn get_uart_output(&self) -> String {
        self.emu.get_uart_output().to_string()
    }

    /// Clear UART output buffer
    pub fn clear_uart_output(&mut self) {
        self.emu.clear_uart_output();
    }

    /// Send a character to UART RX (simulates input)
    pub fn uart_send_char(&mut self, c: char) {
        self.emu.send_uart_byte(c as u8);
    }

    // ===== Additional accessors =====

    /// Get program counter (alias for pc())
    pub fn get_pc(&self) -> u32 {
        self.emu.pc()
    }

    /// Get condition flag (alias for get_c_flag())
    pub fn get_condition_flag(&self) -> bool {
        self.emu.condition_flag()
    }

    /// Get LED value (alias for get_leds())
    pub fn get_led_value(&self) -> u8 {
        self.emu.get_led()
    }

    /// Get instruction count as u32 (truncated from u64)
    pub fn get_instruction_count(&self) -> u32 {
        self.emu.instructions_count() as u32
    }

    /// Read a byte from memory (alias for read_memory())
    pub fn read_byte(&self, addr: u32) -> u8 {
        self.emu.read_byte(addr)
    }

    /// Get the current instruction disassembly (uses EmulatorCore's disassembler)
    pub fn get_current_instruction(&self) -> String {
        if self.emu.is_halted() {
            return "HALTED".to_string();
        }
        let pc = self.emu.pc();
        let (text, _) = self.emu.disassemble_at(pc);
        format!("{:04X}: {}", pc, text)
    }

    /// Get the end address of loaded program (highest address written)
    pub fn get_program_end(&self) -> u32 {
        self.emu.program_end()
    }

    /// Check if should stop for LED output (for animation purposes)
    pub fn should_stop_for_led(&self) -> bool {
        self.emu.cycles() >= 10000
    }
}

// ===== Challenge System =====

/// Get number of available challenges
#[wasm_bindgen]
pub fn get_challenge_count() -> usize {
    get_challenges().len()
}

/// Validate a solution for a challenge
#[wasm_bindgen]
pub fn validate_challenge(challenge_id: usize, source: &str) -> Result<bool, JsValue> {
    let challenges = get_challenges();
    let challenge = challenges
        .iter()
        .find(|c| c.id == challenge_id)
        .ok_or_else(|| JsValue::from_str(&format!("Challenge {} not found", challenge_id)))?;

    let mut assembler = Assembler::new();
    let result = assembler.assemble(source);

    if !result.errors.is_empty() {
        return Err(JsValue::from_str(&result.errors.join("\n")));
    }

    let mut cpu = CpuState::new();
    cpu.load_program(0, &result.bytes);

    let executor = Executor::new();
    match executor.run(&mut cpu, 100000) {
        ExecuteResult::Ok | ExecuteResult::Halted => Ok((challenge.validator)(&cpu)),
        ExecuteResult::InvalidInstruction(byte) => Err(JsValue::from_str(&format!(
            "Invalid instruction: 0x{:02X}",
            byte
        ))),
        ExecuteResult::MemoryError(addr) => Err(JsValue::from_str(&format!(
            "Memory error at address: 0x{:06X}",
            addr
        ))),
    }
}

/// Initialize the WASM module and mount Yew app
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    yew::Renderer::<crate::app::App>::new().render();
}
