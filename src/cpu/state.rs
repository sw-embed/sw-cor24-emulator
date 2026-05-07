//! COR24 CPU state
//!
//! Memory map (MakerLisp COR24):
//!   0x000000 - 0x0FFFFF  SRAM (1 MB on-board)
//!   0x100000 - 0xFDFFFF  Unmapped (addressable, returns 0)
//!   0xFEE000 - 0xFEFFFF  Embedded Block RAM (8 KB window, 3 KB populated)
//!   0xFF0000 - 0xFFFFFF  I/O space

use serde::{Deserialize, Serialize};
use std::fmt;

use super::instruction::Opcode;

/// SRAM size: 1 MB on-board
pub const SRAM_SIZE: usize = 0x100000;

/// EBR (Embedded Block RAM) base address
pub const EBR_BASE: u32 = 0xFEE000;

/// EBR size: 8 KB window (only 3 KB physically populated on MachXO)
pub const EBR_SIZE: usize = 0x2000;

/// I/O region base address
pub const IO_BASE: u32 = 0xFF0000;

/// Default reset address (programs loaded at 0 by monitor)
pub const RESET_ADDRESS: u32 = 0x000000;

/// Stack pointer initial value (top of EBR populated area)
pub const INITIAL_SP: u32 = 0xFEEC00;

// Memory-mapped I/O addresses (24-bit)
/// LED/Switch data register: write bit 0 to control LED D2, read bit 0 for button S2
pub const IO_LEDSWDAT: u32 = 0xFF0000;
/// Interrupt enable register: bit 0 = UART RX interrupt enable
pub const IO_INTENABLE: u32 = 0xFF0010;
/// UART data register: write to transmit, read to receive (auto-acknowledges RX)
pub const IO_UARTDATA: u32 = 0xFF0100;
/// UART status register:
///   bit 0: RX data ready
///   bit 1: CTS active
///   bit 2: RX overflow
///   bit 7: TX busy
pub const IO_UARTSTAT: u32 = 0xFF0101;

/// Kept for backward compatibility but no longer used in I/O dispatch.
/// The MEMORY_SIZE constant is used by app.rs / wasm.rs for the memory viewer.
/// TODO: remove once web UI is updated to use region-based model.
pub const MEMORY_SIZE: usize = SRAM_SIZE;

/// Direction of a UART transaction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UartDirection {
    /// Input: byte sent to CPU via RX (host → CPU)
    Input,
    /// Output: byte transmitted by CPU via TX (CPU → host)
    Output,
}

/// A single UART transaction record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UartLogEntry {
    pub direction: UartDirection,
    pub byte: u8,
    pub instruction: u64,
}

/// Chronological log of all UART input and output
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct UartLog {
    entries: Vec<UartLogEntry>,
}

impl UartLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, direction: UartDirection, byte: u8, instruction: u64) {
        self.entries.push(UartLogEntry {
            direction,
            byte,
            instruction,
        });
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> &[UartLogEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Format the log with coalesced consecutive same-direction bytes.
    /// Echo patterns (alternating input/output of the same byte) are grouped
    /// so you see the input line followed by the output line.
    pub fn format(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        // Group consecutive same-direction entries
        let groups = self.coalesce();

        let mut out = String::new();
        for (dir, bytes) in &groups {
            let label = match dir {
                UartDirection::Input => " IN",
                UartDirection::Output => "OUT",
            };
            out.push_str(&format!("  {}:  \"{}\"", label, escape_bytes(bytes)));
            out.push('\n');
        }
        out
    }

    /// Coalesce entries into groups of (direction, bytes).
    /// Adjacent same-direction entries merge. Single-byte echo pairs
    /// (IN byte, OUT same-byte) are kept separate but not interleaved
    /// further — they naturally group when the program reads-then-writes
    /// in batches.
    fn coalesce(&self) -> Vec<(UartDirection, Vec<u8>)> {
        let mut groups: Vec<(UartDirection, Vec<u8>)> = Vec::new();

        for entry in &self.entries {
            if let Some(last) = groups.last_mut()
                && last.0 == entry.direction
            {
                last.1.push(entry.byte);
                continue;
            }
            groups.push((entry.direction, vec![entry.byte]));
        }

        groups
    }
}

/// Escape bytes for display: show printable ASCII as-is, CR/LF as \r/\n, others as \xHH
fn escape_bytes(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02X}", b)),
        }
    }
    s
}

/// I/O peripheral state
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct IoState {
    /// LED output register (bit 0 = LED D2 pin, active-low: 0=ON, 1=OFF)
    pub leds: u8,
    /// Switch/button input state (bit 0 = button S2, normally high, low when pressed)
    pub switches: u8,
    /// UART transmit buffer (most recent byte sent)
    pub uart_tx: u8,
    /// UART TX busy flag (false = ready to transmit)
    pub uart_tx_busy: bool,
    /// UART TX busy countdown (instructions remaining until TX ready)
    pub uart_tx_countdown: u8,
    /// Number of instructions UART stays busy after a write (default: 10)
    pub uart_tx_busy_cycles: u8,
    /// If true, UART TX busy never clears (for testing poll-before-write)
    pub uart_never_ready: bool,
    /// Count of characters dropped due to writing while TX busy
    pub uart_tx_dropped: u32,
    /// UART receive buffer
    pub uart_rx: u8,
    /// UART receive data ready flag
    pub uart_rx_ready: bool,
    /// UART RX overflow flag
    pub uart_rx_overflow: bool,
    /// Interrupt enable register (bit 0 = UART RX interrupt)
    pub int_enable: u8,
    /// Output text buffer (for UART terminal)
    pub uart_output: String,
    /// Chronological log of all UART input and output
    pub uart_log: UartLog,
}

impl IoState {
    pub fn new() -> Self {
        Self {
            leds: 1,                 // LED off at reset (active-low: 1=OFF)
            switches: 0x01,          // S2 normally high (low = pressed)
            uart_tx_busy_cycles: 10, // realistic: 10 instructions per character
            uart_never_ready: false,
            uart_tx_dropped: 0,
            uart_tx: 0,
            uart_tx_busy: false, // TX starts ready
            uart_tx_countdown: 0,
            uart_rx: 0,
            uart_rx_ready: false,
            uart_rx_overflow: false,
            int_enable: 0,
            uart_output: String::new(),
            uart_log: UartLog::new(),
        }
    }
}

/// A single instruction execution record
#[derive(Clone, Debug)]
pub struct TraceEntry {
    /// Program counter at start of instruction
    pub pc: u32,
    /// Decoded opcode
    pub opcode: Opcode,
    /// Destination register index
    pub ra: u8,
    /// Source register index
    pub rb: u8,
    /// Instruction size in bytes (1, 2, or 4)
    pub size: u8,
    /// 8-bit immediate (for 2-byte and 4-byte instructions)
    pub imm8: u8,
    /// 24-bit immediate (for 4-byte instructions)
    pub imm24: u32,
    /// Register state before execution
    pub regs_before: [u32; 8],
    /// Register state after execution
    pub regs_after: [u32; 8],
    /// Condition flag before
    pub c_before: bool,
    /// Condition flag after
    pub c_after: bool,
    /// SP before (convenience, same as regs_before\[4\])
    pub sp_before: u32,
    /// Instruction count at this point
    pub instruction_num: u64,
}

/// Register names for display
const REG_NAMES: [&str; 8] = ["r0", "r1", "r2", "fp", "sp", "z/c", "iv", "ir"];

impl fmt::Display for TraceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Disassemble the instruction
        let disasm = self.disassemble();

        write!(
            f,
            "{:5} 0x{:06X}  {:<24}",
            self.instruction_num, self.pc, disasm
        )?;

        // Show register changes
        let mut changes = Vec::new();
        for (i, name) in REG_NAMES.iter().enumerate() {
            if self.regs_before[i] != self.regs_after[i] {
                changes.push(format!(
                    "{}:0x{:06X}→0x{:06X}",
                    name, self.regs_before[i], self.regs_after[i]
                ));
            }
        }
        if self.c_before != self.c_after {
            changes.push(format!("c:{}→{}", self.c_before as u8, self.c_after as u8));
        }
        if !changes.is_empty() {
            write!(f, "  [{}]", changes.join(" "))?;
        }

        Ok(())
    }
}

impl TraceEntry {
    /// Disassemble this instruction into a human-readable string
    pub fn disassemble(&self) -> String {
        let ra = REG_NAMES[self.ra as usize];
        let rb = REG_NAMES[self.rb as usize];
        match self.opcode {
            Opcode::AddReg => format!("add     {},{}", ra, rb),
            Opcode::AddImm => format!("add     {},{}", ra, self.imm8 as i8),
            Opcode::And => format!("and     {},{}", ra, rb),
            Opcode::Bra => {
                let offset = self.imm8 as i8;
                let target = (self.pc as i64 + 4 + offset as i64) as u32 & 0xFFFFFF;
                format!("bra     0x{:06X}", target)
            }
            Opcode::Brf => {
                let offset = self.imm8 as i8;
                let target = (self.pc as i64 + 4 + offset as i64) as u32 & 0xFFFFFF;
                format!("brf     0x{:06X}", target)
            }
            Opcode::Brt => {
                let offset = self.imm8 as i8;
                let target = (self.pc as i64 + 4 + offset as i64) as u32 & 0xFFFFFF;
                format!("brt     0x{:06X}", target)
            }
            Opcode::Ceq => format!("ceq     {},{}", ra, rb),
            Opcode::Cls => format!("cls     {},{}", ra, rb),
            Opcode::Clu => format!("clu     {},{}", ra, rb),
            Opcode::Jal => format!("jal     {},({})", ra, rb),
            Opcode::Jmp => format!("jmp     ({})", ra),
            Opcode::La => format!("la      {},0x{:06X}", ra, self.imm24),
            Opcode::Lb => format!("lb      {},{}({})", ra, self.imm8 as i8, rb),
            Opcode::Lbu => format!("lbu     {},{}({})", ra, self.imm8, rb),
            Opcode::Lc => format!("lc      {},{}", ra, self.imm8 as i8),
            Opcode::Lcu => format!("lcu     {},{}", ra, self.imm8),
            Opcode::Lw => format!("lw      {},{}({})", ra, self.imm8 as i8, rb),
            Opcode::Mov => format!("mov     {},{}", ra, rb),
            Opcode::Mul => format!("mul     {},{}", ra, rb),
            Opcode::Or => format!("or      {},{}", ra, rb),
            Opcode::Pop => format!("pop     {}", ra),
            Opcode::Push => format!("push    {}", ra),
            Opcode::Sb => format!("sb      {},{}({})", ra, self.imm8 as i8, rb),
            Opcode::Shl => format!("shl     {},{}", ra, rb),
            Opcode::Sra => format!("sra     {},{}", ra, rb),
            Opcode::Srl => format!("srl     {},{}", ra, rb),
            Opcode::Sub => format!("sub     {},{}", ra, rb),
            Opcode::SubSp => format!("sub     sp,{}", self.imm24),
            Opcode::Sw => format!("sw      {},{}({})", ra, self.imm8 as i8, rb),
            Opcode::Sxt => format!("sxt     {},{}", ra, rb),
            Opcode::Xor => format!("xor     {},{}", ra, rb),
            Opcode::Zxt => format!("zxt     {},{}", ra, rb),
            Opcode::Invalid => format!("??? (0x{:02X})", self.imm8),
        }
    }
}

/// Ring buffer of recent instruction trace entries
#[derive(Clone)]
pub struct TraceBuffer {
    entries: Vec<TraceEntry>,
    head: usize,
    capacity: usize,
    count: usize,
}

impl Default for TraceBuffer {
    fn default() -> Self {
        Self::new(200)
    }
}

impl TraceBuffer {
    /// Create a new trace buffer with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            head: 0,
            capacity,
            count: 0,
        }
    }

    /// Record a trace entry
    pub fn push(&mut self, entry: TraceEntry) {
        if self.entries.len() < self.capacity {
            self.entries.push(entry);
        } else {
            self.entries[self.head] = entry;
        }
        self.head = (self.head + 1) % self.capacity;
        self.count += 1;
    }

    /// Get the last N entries in chronological order
    pub fn last_n(&self, n: usize) -> Vec<&TraceEntry> {
        let len = self.entries.len();
        let n = n.min(len);
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let idx = if len < self.capacity {
                // Not yet wrapped
                len - n + i
            } else {
                (self.head + self.capacity - n + i) % self.capacity
            };
            result.push(&self.entries[idx]);
        }
        result
    }

    /// Get all entries in chronological order
    pub fn all(&self) -> Vec<&TraceEntry> {
        self.last_n(self.entries.len())
    }

    /// Total number of entries ever recorded
    pub fn total_count(&self) -> usize {
        self.count
    }

    /// Number of entries currently in buffer
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.head = 0;
        self.count = 0;
    }

    /// Format the last N entries as a multi-line string
    pub fn format_last(&self, n: usize) -> String {
        let entries = self.last_n(n);
        let mut out = String::new();
        out.push_str(&format!(
            "--- Trace (last {} of {} total) ---\n",
            entries.len(),
            self.count
        ));
        out.push_str(&format!(
            "{:>5} {:>8}  {:<24}  {}\n",
            "#", "PC", "Instruction", "Changes"
        ));
        for entry in &entries {
            out.push_str(&format!("{}\n", entry));
        }
        out
    }
}

/// COR24 CPU state
#[derive(Clone, Serialize, Deserialize)]
pub struct CpuState {
    /// Program counter (24-bit)
    pub pc: u32,
    /// Register file (8 x 24-bit registers)
    pub registers: [u32; 8],
    /// Condition flag (NOT carry — set by ceq/cls/clu, tested by brt/brf)
    pub c: bool,
    /// SRAM (1 MB, 0x000000-0x0FFFFF)
    pub memory: Vec<u8>,
    /// Embedded Block RAM (8 KB, mapped at 0xFEE000-0xFEFFFF)
    pub ebr: Vec<u8>,
    /// Halted flag
    pub halted: bool,
    /// Interrupt-in-service flag (prevents nested interrupts)
    pub intis: bool,
    /// Cycle count
    pub cycles: u64,
    /// Instruction count
    pub instructions: u64,
    /// I/O peripheral state
    pub io: IoState,
    /// Instruction execution trace (ring buffer, not serialized)
    #[serde(skip)]
    pub trace: TraceBuffer,
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuState {
    /// Create a new CPU state with default values
    pub fn new() -> Self {
        let mut state = Self {
            pc: RESET_ADDRESS,
            registers: [0; 8],
            c: false,
            memory: vec![0; SRAM_SIZE],
            ebr: vec![0; EBR_SIZE],
            halted: false,
            intis: false,
            cycles: 0,
            instructions: 0,
            io: IoState::new(),
            trace: TraceBuffer::default(),
        };
        // Initialize stack pointer
        state.registers[4] = INITIAL_SP;
        state
    }

    /// Reset CPU to initial state (preserves memory)
    pub fn reset(&mut self) {
        self.pc = RESET_ADDRESS;
        self.registers = [0; 8];
        self.registers[4] = INITIAL_SP;
        self.c = false;
        self.halted = false;
        self.intis = false;
        self.cycles = 0;
        self.instructions = 0;
        self.io = IoState::new();
        self.trace.clear();
    }

    /// Hard reset (clears memory too)
    pub fn hard_reset(&mut self) {
        self.reset();
        self.memory.fill(0);
        self.ebr.fill(0);
    }

    /// Check if address is in I/O region (0xFF0000-0xFFFFFF)
    fn is_io_addr(addr: u32) -> bool {
        (addr & 0xFF0000) == 0xFF0000
    }

    /// Check if address is in EBR region (0xFEE000-0xFEFFFF)
    fn is_ebr_addr(addr: u32) -> bool {
        addr >= EBR_BASE && addr < EBR_BASE + EBR_SIZE as u32
    }

    /// Read a byte from memory or I/O
    pub fn read_byte(&self, addr: u32) -> u8 {
        let addr = addr & 0xFFFFFF; // Mask to 24 bits

        if Self::is_io_addr(addr) {
            self.read_io(addr)
        } else if Self::is_ebr_addr(addr) {
            self.ebr[(addr - EBR_BASE) as usize]
        } else if (addr as usize) < SRAM_SIZE {
            self.memory[addr as usize]
        } else {
            0 // Unmapped region
        }
    }

    /// Write a byte to memory or I/O
    pub fn write_byte(&mut self, addr: u32, value: u8) {
        let addr = addr & 0xFFFFFF; // Mask to 24 bits

        if Self::is_io_addr(addr) {
            self.write_io(addr, value);
        } else if Self::is_ebr_addr(addr) {
            self.ebr[(addr - EBR_BASE) as usize] = value;
        } else if (addr as usize) < SRAM_SIZE {
            self.memory[addr as usize] = value;
        }
        // Writes to unmapped regions are silently ignored
    }

    /// Read from I/O register
    fn read_io(&self, addr: u32) -> u8 {
        match addr {
            IO_LEDSWDAT => self.io.switches,
            IO_INTENABLE => self.io.int_enable,
            IO_UARTDATA => self.io.uart_rx,
            IO_UARTSTAT => {
                let mut status = 0u8;
                if self.io.uart_rx_ready {
                    status |= 0x01; // Bit 0: RX data ready
                }
                // Bit 1: CTS active (always asserted in emulation)
                status |= 0x02;
                if self.io.uart_rx_overflow {
                    status |= 0x04; // Bit 2: RX overflow
                }
                if self.io.uart_tx_busy {
                    status |= 0x80; // Bit 7: TX busy
                }
                status
            }
            _ => 0, // Unknown I/O address
        }
    }

    /// Read from I/O with side effects (auto-acknowledge)
    /// This is the version that should be called during execution.
    pub fn read_byte_exec(&mut self, addr: u32) -> u8 {
        let addr = addr & 0xFFFFFF;
        if addr == IO_UARTDATA {
            let data = self.io.uart_rx;
            self.io.uart_rx_ready = false; // Auto-acknowledge: reading data clears ready
            data
        } else {
            self.read_byte(addr)
        }
    }

    /// Write to I/O register
    fn write_io(&mut self, addr: u32, value: u8) {
        match addr {
            IO_LEDSWDAT => {
                self.io.leds = value;
            }
            IO_INTENABLE => {
                self.io.int_enable = value;
            }
            IO_UARTDATA => {
                if self.io.uart_tx_busy {
                    // Write while busy — character dropped (hardware would ignore)
                    self.io.uart_tx_dropped += 1;
                } else {
                    self.io.uart_tx = value;
                    if value != 0 {
                        self.io.uart_output.push(value as char);
                    }
                    self.io
                        .uart_log
                        .push(UartDirection::Output, value, self.instructions);
                    // TX busy for N cycles (simulates transmission time)
                    if self.io.uart_tx_busy_cycles > 0 {
                        self.io.uart_tx_busy = true;
                        self.io.uart_tx_countdown = self.io.uart_tx_busy_cycles;
                    }
                    // cycles=0 means instant ready (legacy/test mode)
                }
            }
            IO_UARTSTAT if value & 0x04 != 0 => {
                // Writing bit 2 of status clears the RX overflow flag.
                self.io.uart_rx_overflow = false;
            }
            _ => {} // Ignore unknown I/O address
        }
    }

    /// Tick UART timers — call after each instruction execution
    pub fn uart_tick(&mut self) {
        if self.io.uart_never_ready {
            // TX stays busy forever — tests that programs poll before writing
            return;
        }
        if self.io.uart_tx_countdown > 0 {
            self.io.uart_tx_countdown -= 1;
            if self.io.uart_tx_countdown == 0 {
                self.io.uart_tx_busy = false;
            }
        }
    }

    /// Send a character to UART RX (simulates external input)
    pub fn uart_send_rx(&mut self, ch: u8) {
        if self.io.uart_rx_ready {
            self.io.uart_rx_overflow = true; // Previous data not read
        }
        self.io.uart_rx = ch;
        self.io.uart_rx_ready = true;
        self.io
            .uart_log
            .push(UartDirection::Input, ch, self.instructions);
    }

    /// Read a 24-bit word from memory (little-endian)
    pub fn read_word(&self, addr: u32) -> u32 {
        let b0 = self.read_byte(addr) as u32;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u32;
        let b2 = self.read_byte(addr.wrapping_add(2)) as u32;
        b0 | (b1 << 8) | (b2 << 16)
    }

    /// Write a 24-bit word to memory (little-endian)
    pub fn write_word(&mut self, addr: u32, value: u32) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(1), ((value >> 8) & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(2), ((value >> 16) & 0xFF) as u8);
    }

    /// Get register value (masked to 24 bits)
    pub fn get_reg(&self, reg: u8) -> u32 {
        self.registers[(reg & 0x07) as usize] & 0xFFFFFF
    }

    /// Set register value (masked to 24 bits)
    pub fn set_reg(&mut self, reg: u8, value: u32) {
        self.registers[(reg & 0x07) as usize] = value & 0xFFFFFF;
    }

    /// Sign extend 8-bit to 24-bit
    pub fn sign_extend_8(value: u8) -> u32 {
        if value & 0x80 != 0 {
            0xFFFF00 | (value as u32)
        } else {
            value as u32
        }
    }

    /// Check if an interrupt should fire
    /// Hardware: ireq = uart_rxrdy && ienab; irqis = ireq && !intis
    pub fn interrupt_pending(&self) -> bool {
        self.io.uart_rx_ready && (self.io.int_enable & 0x01 != 0) && !self.intis
    }

    /// Mask to 24 bits
    pub fn mask_24(value: u32) -> u32 {
        value & 0xFFFFFF
    }

    /// Load program into memory at given address
    pub fn load_program(&mut self, start_addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write_byte(start_addr + i as u32, byte);
        }
    }
}

/// Instruction decode ROM
/// Maps 8-bit instruction bytes to 12-bit decoded values: [opcode(5):ra(3):rb(3)]
/// Uses the const DECODE_ROM extracted from dis_rom.v
#[derive(Clone)]
pub struct DecodeRom;

impl Default for DecodeRom {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeRom {
    /// Create decode ROM (uses static const array)
    pub fn new() -> Self {
        Self
    }

    /// Decode an instruction byte
    pub fn decode(&self, byte: u8) -> u16 {
        crate::cpu::decode_rom::DECODE_ROM[byte as usize]
    }

    /// Check if an instruction byte is valid
    pub fn is_valid(&self, byte: u8) -> bool {
        crate::cpu::decode_rom::DECODE_ROM[byte as usize] != 0xFFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Initial State Tests ==========

    #[test]
    fn test_initial_sp_is_feec00() {
        let cpu = CpuState::new();
        assert_eq!(cpu.get_reg(4), 0xFEEC00, "SP must init to 0xFEEC00");
    }

    #[test]
    fn test_cpu_state_new() {
        let cpu = CpuState::new();
        assert_eq!(cpu.pc, RESET_ADDRESS);
        assert_eq!(cpu.registers[4], INITIAL_SP);
        assert!(!cpu.halted);
    }

    // ========== Region-Based Memory Tests ==========

    #[test]
    fn test_sram_region() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0x000100, 0x42);
        assert_eq!(cpu.read_byte(0x000100), 0x42);
        // Must NOT alias to EBR
        assert_eq!(
            cpu.read_byte(0xFEE100),
            0x00,
            "SRAM must not alias into EBR"
        );
    }

    #[test]
    fn test_ebr_region() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFEE000, 0xAA);
        assert_eq!(cpu.read_byte(0xFEE000), 0xAA);
        // Must NOT alias to SRAM
        assert_eq!(
            cpu.read_byte(0x000000),
            0x00,
            "EBR must not alias into SRAM"
        );
    }

    #[test]
    fn test_ebr_stack_area() {
        let mut cpu = CpuState::new();
        // Write a word just below the initial SP
        cpu.write_word(0xFEEBFD, 0x123456);
        assert_eq!(cpu.read_word(0xFEEBFD), 0x123456);
    }

    #[test]
    fn test_io_region_not_in_sram() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFF0000, 0x01); // LED write
        // Must not appear in SRAM
        assert_eq!(cpu.read_byte(0x000000), 0x00);
    }

    #[test]
    fn test_unmapped_memory_returns_zero() {
        let cpu = CpuState::new();
        assert_eq!(cpu.read_byte(0x500000), 0x00, "Unmapped region returns 0");
    }

    #[test]
    fn test_sram_top_boundary() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0x0FFFFF, 0xBB); // Last SRAM byte
        assert_eq!(cpu.read_byte(0x0FFFFF), 0xBB);
        // Address 0x100000 is unmapped
        assert_eq!(cpu.read_byte(0x100000), 0x00);
    }

    #[test]
    fn test_memory_operations() {
        let mut cpu = CpuState::new();

        cpu.write_byte(0x100, 0x42);
        assert_eq!(cpu.read_byte(0x100), 0x42);

        cpu.write_word(0x200, 0x123456);
        assert_eq!(cpu.read_word(0x200), 0x123456);
    }

    // ========== UART Address Tests ==========

    #[test]
    fn test_uart_data_at_ff0100() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFF0100, b'H');
        assert_eq!(cpu.io.uart_tx, b'H');
        assert!(cpu.io.uart_output.contains('H'));
    }

    #[test]
    fn test_uart_status_at_ff0101() {
        let cpu = CpuState::new();
        let status = cpu.read_byte(0xFF0101);
        // TX not busy (bit 7 = 0), CTS active (bit 1 = 1), no RX data (bit 0 = 0)
        assert_eq!(status & 0x82, 0x02, "CTS=1, TX not busy");
    }

    #[test]
    fn test_uart_status_rx_ready() {
        let mut cpu = CpuState::new();
        cpu.io.uart_rx = b'A';
        cpu.io.uart_rx_ready = true;
        let status = cpu.read_byte(0xFF0101);
        assert_eq!(status & 0x01, 0x01, "RX ready bit 0 set");
    }

    #[test]
    fn test_uart_status_tx_busy() {
        let mut cpu = CpuState::new();
        cpu.io.uart_tx_busy = true;
        let status = cpu.read_byte(0xFF0101);
        assert_eq!(status & 0x80, 0x80, "TX busy bit 7 set");
    }

    #[test]
    fn test_uart_old_address_not_mapped() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFFFF00, b'X');
        assert_ne!(cpu.io.uart_tx, b'X', "Old UART address must not work");
    }

    #[test]
    fn test_uart_read_clears_rx_ready() {
        let mut cpu = CpuState::new();
        cpu.io.uart_rx = b'Z';
        cpu.io.uart_rx_ready = true;
        let data = cpu.read_byte_exec(0xFF0100);
        assert_eq!(data, b'Z');
        assert!(
            !cpu.io.uart_rx_ready,
            "Reading UART data auto-clears RX ready"
        );
    }

    #[test]
    fn test_uart_send_rx() {
        let mut cpu = CpuState::new();
        cpu.uart_send_rx(b'A');
        assert_eq!(cpu.io.uart_rx, b'A');
        assert!(cpu.io.uart_rx_ready);
        assert!(!cpu.io.uart_rx_overflow);

        // Send another without reading — should overflow
        cpu.uart_send_rx(b'B');
        assert_eq!(cpu.io.uart_rx, b'B');
        assert!(cpu.io.uart_rx_ready);
        assert!(cpu.io.uart_rx_overflow);
    }

    // ========== LED/Switch Tests ==========

    #[test]
    fn test_led_write_bit0() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFF0000, 0x01);
        assert_eq!(cpu.io.leds, 0x01);
    }

    #[test]
    fn test_switch_default_high() {
        let cpu = CpuState::new();
        // Button S2 normally high
        assert_eq!(cpu.read_byte(0xFF0000) & 0x01, 0x01);
    }

    // ========== Interrupt Enable Tests ==========

    #[test]
    fn test_interrupt_enable_register() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFF0010, 0x01);
        assert_eq!(cpu.io.int_enable, 0x01);
        assert_eq!(cpu.read_byte(0xFF0010), 0x01);
    }

    // ========== Sieve putchr UART poll simulation ==========

    #[test]
    fn test_sieve_putchr_uart_poll() {
        // Simulate what _putchr does:
        // la r2, -65280  → r2 = 0xFF0100 (UART base)
        // lb r0,1(r2)    → read status at 0xFF0101
        // lc r1,2; and r0,r1 → test bit 1 (CTS)
        let cpu = CpuState::new();

        // Read UART status at 0xFF0101
        let status = cpu.read_byte(0xFF0101);
        // CTS should be active (bit 1 = 1) for putchr to proceed
        assert!(
            status & 0x02 != 0,
            "CTS must be active for putchr to proceed"
        );
        // TX not busy (bit 7 = 0) so cls r0,z won't loop
        assert!(status & 0x80 == 0, "TX must not be busy");
    }

    // ========== Push/Pop at correct SP ==========

    #[test]
    fn test_push_pop_at_feec00() {
        let mut cpu = CpuState::new();
        assert_eq!(cpu.get_reg(4), 0xFEEC00);

        // Simulate push: sp -= 3, store word
        let val = 0x123456u32;
        let sp = cpu.get_reg(4) - 3;
        cpu.set_reg(4, sp);
        cpu.write_word(sp, val);

        assert_eq!(cpu.get_reg(4), 0xFEEBFD);
        assert_eq!(cpu.read_word(0xFEEBFD), 0x123456);

        // Simulate pop: read word, sp += 3
        let popped = cpu.read_word(cpu.get_reg(4));
        cpu.set_reg(4, cpu.get_reg(4) + 3);
        assert_eq!(popped, 0x123456);
        assert_eq!(cpu.get_reg(4), 0xFEEC00);
    }

    #[test]
    fn test_8kb_stack_push_pop_at_ff0000() {
        let mut cpu = CpuState::new();
        cpu.set_reg(4, 0xFF0000); // 8 KB stack: SP at top of full EBR window

        // Push a word at the top of the 8 KB region
        let val = 0xABCDEFu32;
        let sp = cpu.get_reg(4) - 3;
        cpu.set_reg(4, sp);
        cpu.write_word(sp, val);
        assert_eq!(cpu.get_reg(4), 0xFEFFFD);
        assert_eq!(cpu.read_word(0xFEFFFD), 0xABCDEF);

        // Pop it back
        let popped = cpu.read_word(cpu.get_reg(4));
        cpu.set_reg(4, cpu.get_reg(4) + 3);
        assert_eq!(popped, 0xABCDEF);
        assert_eq!(cpu.get_reg(4), 0xFF0000);
    }

    #[test]
    fn test_8kb_stack_deep_pushes() {
        // Verify the full 8 KB EBR region (0xFEE000-0xFEFFFF) is usable as stack
        let mut cpu = CpuState::new();
        cpu.set_reg(4, 0xFF0000);

        // Push ~2700 words (8100 bytes > 3 KB, proving the upper 5 KB is writable)
        let n = 2700u32;
        for i in 0..n {
            let sp = cpu.get_reg(4) - 3;
            cpu.set_reg(4, sp);
            cpu.write_word(sp, i);
        }

        // SP should be 0xFF0000 - (2700 * 3) = 0xFF0000 - 0x1F7C = 0xFEE084
        assert_eq!(cpu.get_reg(4), 0xFF0000 - n * 3);
        assert!(cpu.get_reg(4) >= EBR_BASE, "SP should still be within EBR");

        // Verify all values pop back correctly (LIFO order)
        for i in (0..n).rev() {
            let popped = cpu.read_word(cpu.get_reg(4));
            cpu.set_reg(4, cpu.get_reg(4) + 3);
            assert_eq!(popped, i, "Stack corruption at depth {}", n - i);
        }
        assert_eq!(cpu.get_reg(4), 0xFF0000);
    }

    #[test]
    fn test_8kb_stack_no_io_collision() {
        // Writing to the top of EBR (just below 0xFF0000) must not touch I/O space
        let mut cpu = CpuState::new();
        cpu.set_reg(4, 0xFF0000);

        // Snapshot I/O state before stack writes
        let led_before = cpu.read_byte(0xFF0000); // button S2 default = 1 (released)
        let uart_before = cpu.read_byte(0xFF0100);

        // Push at 0xFEFFFD-0xFEFFFF (top 3 bytes of EBR)
        let sp = cpu.get_reg(4) - 3;
        cpu.set_reg(4, sp);
        cpu.write_word(sp, 0xFFFFFF);

        // I/O region should be unchanged
        assert_eq!(
            cpu.read_byte(0xFF0000),
            led_before,
            "I/O LED register must not be corrupted by stack"
        );
        assert_eq!(
            cpu.read_byte(0xFF0100),
            uart_before,
            "I/O UART data must not be corrupted by stack"
        );
    }

    // ========== Sign Extension ==========

    #[test]
    fn test_sign_extend() {
        assert_eq!(CpuState::sign_extend_8(0x7F), 0x00007F);
        assert_eq!(CpuState::sign_extend_8(0x80), 0xFFFF80);
        assert_eq!(CpuState::sign_extend_8(0xFF), 0xFFFFFF);
    }

    // ========== UART Log Tests ==========

    #[test]
    fn test_uart_log_empty() {
        let log = UartLog::new();
        assert!(log.is_empty());
        assert_eq!(log.format(), "");
    }

    #[test]
    fn test_uart_log_output_only() {
        let mut log = UartLog::new();
        for &b in b"Hi\n" {
            log.push(UartDirection::Output, b, 0);
        }
        assert_eq!(log.format(), "  OUT:  \"Hi\\n\"\n");
    }

    #[test]
    fn test_uart_log_input_only() {
        let mut log = UartLog::new();
        for &b in b"abc" {
            log.push(UartDirection::Input, b, 0);
        }
        assert_eq!(log.format(), "   IN:  \"abc\"\n");
    }

    #[test]
    fn test_uart_log_coalesce_same_direction() {
        let mut log = UartLog::new();
        log.push(UartDirection::Output, b'H', 1);
        log.push(UartDirection::Output, b'i', 2);
        log.push(UartDirection::Output, b'\n', 3);
        let groups = log.coalesce();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1, b"Hi\n");
    }

    #[test]
    fn test_uart_log_echo_pattern() {
        // Simulates: prompt "?" output, then input 'a', then echo output 'A'
        let mut log = UartLog::new();
        log.push(UartDirection::Output, b'?', 10);
        log.push(UartDirection::Input, b'a', 20);
        log.push(UartDirection::Output, b'A', 30);
        let formatted = log.format();
        assert_eq!(formatted, "  OUT:  \"?\"\n   IN:  \"a\"\n  OUT:  \"A\"\n");
    }

    #[test]
    fn test_uart_log_batch_echo() {
        // Input "abc" then output "ABC" — no interleaving
        let mut log = UartLog::new();
        for &b in b"abc" {
            log.push(UartDirection::Input, b, 0);
        }
        for &b in b"ABC" {
            log.push(UartDirection::Output, b, 0);
        }
        let groups = log.coalesce();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], (UartDirection::Input, b"abc".to_vec()));
        assert_eq!(groups[1], (UartDirection::Output, b"ABC".to_vec()));
    }

    #[test]
    fn test_uart_log_interleaved_echo() {
        // Character-by-character echo: in 'a', out 'A', in 'b', out 'B'
        let mut log = UartLog::new();
        log.push(UartDirection::Input, b'a', 1);
        log.push(UartDirection::Output, b'A', 2);
        log.push(UartDirection::Input, b'b', 3);
        log.push(UartDirection::Output, b'B', 4);
        let groups = log.coalesce();
        assert_eq!(groups.len(), 4); // each is its own group
        let formatted = log.format();
        assert!(formatted.contains("IN:  \"a\""));
        assert!(formatted.contains("OUT:  \"A\""));
    }

    #[test]
    fn test_uart_log_clear() {
        let mut log = UartLog::new();
        log.push(UartDirection::Output, b'x', 0);
        assert!(!log.is_empty());
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn test_uart_log_escape_control_chars() {
        let mut log = UartLog::new();
        log.push(UartDirection::Output, b'\r', 0);
        log.push(UartDirection::Output, b'\n', 0);
        log.push(UartDirection::Output, b'\t', 0);
        log.push(UartDirection::Output, 0x01, 0);
        assert_eq!(log.format(), "  OUT:  \"\\r\\n\\t\\x01\"\n");
    }

    #[test]
    fn test_uart_log_recorded_on_tx() {
        let mut cpu = CpuState::new();
        cpu.io.uart_tx_busy_cycles = 0; // instant TX
        cpu.write_byte(IO_UARTDATA, b'H');
        cpu.write_byte(IO_UARTDATA, b'i');
        assert_eq!(cpu.io.uart_log.entries().len(), 2);
        assert_eq!(
            cpu.io.uart_log.entries()[0].direction,
            UartDirection::Output
        );
        assert_eq!(cpu.io.uart_log.entries()[0].byte, b'H');
    }

    #[test]
    fn test_uart_log_recorded_on_rx() {
        let mut cpu = CpuState::new();
        cpu.uart_send_rx(b'A');
        assert_eq!(cpu.io.uart_log.entries().len(), 1);
        assert_eq!(cpu.io.uart_log.entries()[0].direction, UartDirection::Input);
        assert_eq!(cpu.io.uart_log.entries()[0].byte, b'A');
    }

    #[test]
    fn test_uart_log_cleared_on_reset() {
        let mut cpu = CpuState::new();
        cpu.io.uart_tx_busy_cycles = 0;
        cpu.write_byte(IO_UARTDATA, b'X');
        cpu.uart_send_rx(b'Y');
        assert_eq!(cpu.io.uart_log.entries().len(), 2);
        cpu.reset();
        assert!(cpu.io.uart_log.is_empty());
    }
}
