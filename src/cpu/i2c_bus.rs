//! I2C bus state machine.
//!
//! Edge detection over (SCL, SDA) line transitions — START, STOP, byte
//! shift on SCL rising edge, ACK clock — without any device-side logic.
//! Devices (slave-side pull-down, byte response) wire in at step B.1
//! via the I2cDevice trait.
//!
//! With no devices attached the wired-AND collapses to the master
//! driver, so every transaction NAKs (SDA=1 = NAK). The phase is still
//! tracked and observable through `cpu.io.i2c`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum I2cPhase {
    /// Bus quiet — both lines high.
    #[default]
    Idle,
    /// Saw START; awaiting first SCL rising edge of the address byte.
    Started,
    /// Shifting in a byte from master. `bits` accumulates MSB-first;
    /// `n` is the count of bits collected (1..=7).
    RxByte { bits: u8, n: u8 },
    /// 8 bits collected; the next SCL rising edge clocks the ACK bit
    /// the slave drives (or NAK with no slave).
    AckMasterToSlave,
    /// Slave is shifting a byte out to the master.
    TxByte { bits: u8, n: u8 },
    /// 8 bits read; master ACKs to continue or NAKs to end the read.
    AckSlaveToMaster,
    /// Saw STOP; resolves to Idle on the next bus event.
    Stopped,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum I2cDir {
    #[default]
    Write,
    Read,
}

/// I2C bus state — phase, addressing context, edge-detection memory.
///
/// The master-side line state lives in `IoState::master_scl` /
/// `master_sda` (added in step A.2). This struct only tracks
/// protocol-level state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct I2cBusState {
    pub phase: I2cPhase,
    /// 7-bit slave address, set when the address byte completes.
    /// Cleared on START / STOP.
    pub current_target: Option<u8>,
    pub current_dir: I2cDir,
    /// Most recent fully-decoded byte (address byte or write data).
    /// Cleared on START.
    pub last_byte: Option<u8>,
    /// Sticky 7-bit address last observed on the bus — survives STOP so
    /// tests and observers can assert "the bus addressed device X at
    /// some point during this run."
    pub last_addressed: Option<u8>,
    /// Count of START conditions seen — useful for smoke-tests that
    /// want to confirm the state machine made progress.
    pub transactions: u32,
    /// Effective line state seen at the last `step()` call. Used for
    /// edge detection.
    pub last_scl: bool,
    pub last_sda: bool,
}

impl I2cBusState {
    pub fn new() -> Self {
        Self {
            phase: I2cPhase::Idle,
            current_target: None,
            current_dir: I2cDir::Write,
            last_byte: None,
            last_addressed: None,
            transactions: 0,
            last_scl: true,
            last_sda: true,
        }
    }

    /// Advance the state machine after the master writes either I2C
    /// line. `new_scl` / `new_sda` are the *effective* lines (master &
    /// !slave_pull). With no devices, slave_pull is always false.
    pub fn step(&mut self, new_scl: bool, new_sda: bool) {
        let prev_scl = self.last_scl;
        let prev_sda = self.last_sda;

        // Order matters: START/STOP edges (which require SCL high
        // throughout) must be detected before SCL-rise sampling.
        if new_scl && prev_scl && prev_sda && !new_sda {
            // START: SDA falls while SCL is high. Repeated START from a
            // mid-transaction phase lands here too.
            self.phase = I2cPhase::Started;
            self.current_target = None;
            self.last_byte = None;
            self.transactions = self.transactions.saturating_add(1);
        } else if new_scl && prev_scl && !prev_sda && new_sda {
            // STOP: SDA rises while SCL is high.
            self.phase = I2cPhase::Stopped;
            self.current_target = None;
        } else if new_scl && !prev_scl {
            // SCL rising edge: the bus samples SDA at this moment.
            self.on_scl_rise(new_sda);
        }

        // Stopped settles to Idle on the next event of any kind.
        if matches!(self.phase, I2cPhase::Stopped)
            && (new_scl != prev_scl || new_sda != prev_sda)
            && self.phase == I2cPhase::Stopped
            && new_scl
            && new_sda
        {
            // Both lines released high after STOP — return to Idle.
            self.phase = I2cPhase::Idle;
        }

        self.last_scl = new_scl;
        self.last_sda = new_sda;
    }

    fn on_scl_rise(&mut self, sda: bool) {
        let sda_bit = sda as u8;
        match self.phase {
            I2cPhase::Idle => {
                // Spurious clock with no START — ignore.
            }
            I2cPhase::Started => {
                self.phase = I2cPhase::RxByte { bits: sda_bit, n: 1 };
            }
            I2cPhase::RxByte { bits, n } => {
                let new_bits = (bits << 1) | sda_bit;
                let new_n = n + 1;
                if new_n == 8 {
                    self.last_byte = Some(new_bits);
                    if self.current_target.is_none() {
                        // First byte after START — this is the address byte.
                        let addr = new_bits >> 1;
                        self.current_target = Some(addr);
                        self.last_addressed = Some(addr);
                        self.current_dir = if new_bits & 1 == 1 {
                            I2cDir::Read
                        } else {
                            I2cDir::Write
                        };
                    }
                    self.phase = I2cPhase::AckMasterToSlave;
                } else {
                    self.phase = I2cPhase::RxByte { bits: new_bits, n: new_n };
                }
            }
            I2cPhase::AckMasterToSlave => {
                // Master sampled the ACK bit (always NAK with no device).
                // Move into the next byte slot based on direction.
                self.phase = match self.current_dir {
                    I2cDir::Write => I2cPhase::RxByte { bits: 0, n: 0 },
                    I2cDir::Read => I2cPhase::TxByte { bits: 0, n: 0 },
                };
            }
            I2cPhase::TxByte { bits, n } => {
                // Slave-driven byte — without a device, every bit reads as 1
                // (the line is just released high). Track the count.
                let new_bits = (bits << 1) | sda_bit;
                let new_n = n + 1;
                if new_n == 8 {
                    self.last_byte = Some(new_bits);
                    self.phase = I2cPhase::AckSlaveToMaster;
                } else {
                    self.phase = I2cPhase::TxByte { bits: new_bits, n: new_n };
                }
            }
            I2cPhase::AckSlaveToMaster => {
                // Master ACK (SDA=0) → another read byte; NAK (SDA=1) →
                // master will send STOP/repeated START next. Either way,
                // start a fresh TxByte slot; STOP/START will override.
                self.phase = I2cPhase::TxByte { bits: 0, n: 0 };
            }
            I2cPhase::Stopped => {
                // SCL rise after STOP shouldn't happen on a real bus;
                // treat as a return to Idle.
                self.phase = I2cPhase::Idle;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::state::{CpuState, IO_I2C_SCL, IO_I2C_SDA};

    /// Drive one bit by setting SDA, pulsing SCL high then low.
    fn clock_bit(cpu: &mut CpuState, sda_bit: u8) {
        cpu.write_byte(IO_I2C_SDA, sda_bit);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SCL, 0);
    }

    /// Drive an 8-bit byte MSB-first followed by an ACK clock with SDA
    /// released. With no device, the slave reads as NAK (1).
    fn send_byte(cpu: &mut CpuState, byte: u8) {
        for i in (0..8).rev() {
            clock_bit(cpu, (byte >> i) & 1);
        }
        // ACK slot: master releases SDA, pulses SCL.
        clock_bit(cpu, 1);
    }

    /// I2C START: SDA high, SCL high, then SDA falls.
    fn start(cpu: &mut CpuState) {
        cpu.write_byte(IO_I2C_SDA, 1);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SDA, 0);
        cpu.write_byte(IO_I2C_SCL, 0);
    }

    /// I2C STOP: SDA low, SCL high, then SDA rises.
    fn stop(cpu: &mut CpuState) {
        cpu.write_byte(IO_I2C_SDA, 0);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SDA, 1);
    }

    #[test]
    fn idle_stays_idle_without_start() {
        let mut cpu = CpuState::new();
        // Pulse SCL with no START condition — should remain Idle.
        cpu.write_byte(IO_I2C_SCL, 0);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SCL, 0);
        assert_eq!(cpu.io.i2c.phase, I2cPhase::Idle);
        assert_eq!(cpu.io.i2c.current_target, None);
    }

    #[test]
    fn start_then_address_byte_decoded() {
        let mut cpu = CpuState::new();
        start(&mut cpu);
        assert_eq!(cpu.io.i2c.phase, I2cPhase::Started);

        // Address 0x4A, write direction → byte = 0x94
        send_byte(&mut cpu, 0x94);

        assert_eq!(cpu.io.i2c.current_target, Some(0x4A));
        assert_eq!(cpu.io.i2c.current_dir, I2cDir::Write);
        assert_eq!(cpu.io.i2c.last_byte, Some(0x94));
    }

    #[test]
    fn read_direction_address_decoded() {
        let mut cpu = CpuState::new();
        start(&mut cpu);
        // Address 0x4A, read direction → byte = 0x95
        send_byte(&mut cpu, 0x95);

        assert_eq!(cpu.io.i2c.current_target, Some(0x4A));
        assert_eq!(cpu.io.i2c.current_dir, I2cDir::Read);
    }

    #[test]
    fn stop_returns_to_idle() {
        let mut cpu = CpuState::new();
        start(&mut cpu);
        send_byte(&mut cpu, 0x94);
        stop(&mut cpu);
        // After STOP we should be Stopped or Idle; either is fine for
        // observability — the key is current_target is cleared.
        assert!(matches!(
            cpu.io.i2c.phase,
            I2cPhase::Stopped | I2cPhase::Idle
        ));
        assert_eq!(cpu.io.i2c.current_target, None);
        // last_addressed survives STOP so observers can see "the bus
        // addressed 0x4A at some point during this run."
        assert_eq!(cpu.io.i2c.last_addressed, Some(0x4A));
        assert_eq!(cpu.io.i2c.transactions, 1);
    }

    #[test]
    fn repeated_start_resets_address() {
        let mut cpu = CpuState::new();
        start(&mut cpu);
        send_byte(&mut cpu, 0x94); // addr 0x4A write
        // No STOP — repeated START mid-transaction.
        start(&mut cpu);
        assert_eq!(cpu.io.i2c.phase, I2cPhase::Started);
        assert_eq!(cpu.io.i2c.current_target, None);
        assert_eq!(cpu.io.i2c.transactions, 2);

        send_byte(&mut cpu, 0x95); // addr 0x4A read
        assert_eq!(cpu.io.i2c.current_target, Some(0x4A));
        assert_eq!(cpu.io.i2c.current_dir, I2cDir::Read);
    }

    #[test]
    fn two_consecutive_write_bytes_decoded() {
        let mut cpu = CpuState::new();
        start(&mut cpu);
        send_byte(&mut cpu, 0x94); // addr byte
        assert_eq!(cpu.io.i2c.last_byte, Some(0x94));

        send_byte(&mut cpu, 0x01); // first data byte
        assert_eq!(cpu.io.i2c.last_byte, Some(0x01));

        send_byte(&mut cpu, 0x20); // second data byte
        assert_eq!(cpu.io.i2c.last_byte, Some(0x20));

        // Address sticky throughout the write transaction.
        assert_eq!(cpu.io.i2c.current_target, Some(0x4A));
        assert_eq!(cpu.io.i2c.current_dir, I2cDir::Write);
    }
}
