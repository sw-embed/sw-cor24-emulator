//! COR24 CPU state
//!
//! Memory map (MakerLisp COR24):
//!   0x000000 - 0x0FFFFF  SRAM (1 MB on-board)
//!   0x100000 - 0xFDFFFF  Unmapped (addressable, returns 0)
//!   0xFEE000 - 0xFEFFFF  Embedded Block RAM (8 KB window, 3 KB populated)
//!   0xFF0000 - 0xFFFFFF  I/O space

use serde::{Deserialize, Serialize};

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

/// I/O peripheral state
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct IoState {
    /// LED output state (bit 0 = LED D2, active low on hardware)
    pub leds: u8,
    /// Switch/button input state (bit 0 = button S2, normally high, low when pressed)
    pub switches: u8,
    /// UART transmit buffer (most recent byte sent)
    pub uart_tx: u8,
    /// UART TX busy flag (false = ready to transmit)
    pub uart_tx_busy: bool,
    /// UART TX busy countdown (instructions remaining until TX ready)
    pub uart_tx_countdown: u8,
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
}

impl IoState {
    pub fn new() -> Self {
        Self {
            leds: 0,
            switches: 0x01, // S2 normally high (low = pressed)
            uart_tx: 0,
            uart_tx_busy: false, // TX starts ready
            uart_tx_countdown: 0,
            uart_rx: 0,
            uart_rx_ready: false,
            uart_rx_overflow: false,
            int_enable: 0,
            uart_output: String::new(),
        }
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
    /// Cycle count
    pub cycles: u64,
    /// Instruction count
    pub instructions: u64,
    /// I/O peripheral state
    pub io: IoState,
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
            cycles: 0,
            instructions: 0,
            io: IoState::new(),
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
        self.cycles = 0;
        self.instructions = 0;
        self.io = IoState::new();
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
                self.io.uart_tx = value;
                // Append to output buffer (for terminal display)
                if value != 0 {
                    self.io.uart_output.push(value as char);
                }
                // TX busy for 1 instruction cycle (simulates transmission)
                self.io.uart_tx_busy = true;
                self.io.uart_tx_countdown = 1;
            }
            IO_UARTSTAT => {
                // Writing to status can clear overflow flag
                if value & 0x04 != 0 {
                    self.io.uart_rx_overflow = false;
                }
            }
            _ => {} // Ignore unknown I/O address
        }
    }

    /// Tick UART timers — call after each instruction execution
    pub fn uart_tick(&mut self) {
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
        assert_eq!(cpu.read_byte(0xFEE100), 0x00, "SRAM must not alias into EBR");
    }

    #[test]
    fn test_ebr_region() {
        let mut cpu = CpuState::new();
        cpu.write_byte(0xFEE000, 0xAA);
        assert_eq!(cpu.read_byte(0xFEE000), 0xAA);
        // Must NOT alias to SRAM
        assert_eq!(cpu.read_byte(0x000000), 0x00, "EBR must not alias into SRAM");
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
        assert!(!cpu.io.uart_rx_ready, "Reading UART data auto-clears RX ready");
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
        assert!(status & 0x02 != 0, "CTS must be active for putchr to proceed");
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

    // ========== Sign Extension ==========

    #[test]
    fn test_sign_extend() {
        assert_eq!(CpuState::sign_extend_8(0x7F), 0x00007F);
        assert_eq!(CpuState::sign_extend_8(0x80), 0xFFFF80);
        assert_eq!(CpuState::sign_extend_8(0xFF), 0xFFFFFF);
    }
}
