//! I2C bus state machine.
//!
//! Edge detection over (SCL, SDA) line transitions — START, STOP, byte
//! shift on SCL rising edge, ACK clock — combined with device dispatch
//! through the per-bus address-routing table. The slave-side SDA
//! pull-down (`slave_sda_pull`) is wired into the read of `IO_I2C_SDA`
//! so the master sees ACKs and slave-driven read bytes through the
//! same wired-AND a real I2C bus presents.
//!
//! Phase changes happen on SCL rises (sampling edge); slave-line pulls
//! are computed on SCL falls (between sampling edges) so they hold
//! across the upcoming SCL high period and are released by the next
//! fall — matching real I2C timing.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::peripherals::i2c::device::Ack;
use crate::peripherals::i2c::registry::AddressMap;

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
    /// 8 bits collected; the 9th SCL pulse is the ACK clock the slave
    /// drives (or NAK with no slave).
    AckMasterToSlave,
    /// Slave is shifting a byte out to the master. `bits` accumulates
    /// what was sent (for `last_byte`); `n` is bits sent (1..=7).
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

/// I2C bus state — phase, addressing context, edge-detection memory,
/// device-routing table, and slave-line drive state.
#[derive(Clone, Default, Serialize, Deserialize)]
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
    /// Effective SCL/SDA seen at the last `step()` call. Used for
    /// edge detection.
    pub last_scl: bool,
    pub last_sda: bool,
    /// Whether the slave is actively pulling SDA low. Read by
    /// `state.rs::read_io` to compute the wired-AND'ed effective line.
    /// Skipped from serde — runtime/transport state.
    #[serde(skip)]
    pub slave_sda_pull: bool,
    /// Byte the slave has buffered to send during the next TxByte. The
    /// MSB is shifted out on each TxByte SCL rise.
    #[serde(skip)]
    tx_byte: u8,
    /// ACK decision for the in-progress AckMasterToSlave clock; computed
    /// on the byte-completing rise, applied on the next fall.
    #[serde(skip)]
    pending_ack: Ack,
    /// Per-bus address-routing table. Cloning the bus state shares the
    /// table (Arc), so attached devices survive snapshot/restore as
    /// far as in-memory state is concerned.
    #[serde(skip, default)]
    pub addresses: AddressMap,
}

impl fmt::Debug for I2cBusState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I2cBusState")
            .field("phase", &self.phase)
            .field("current_target", &self.current_target)
            .field("current_dir", &self.current_dir)
            .field("last_byte", &self.last_byte)
            .field("last_addressed", &self.last_addressed)
            .field("transactions", &self.transactions)
            .field("last_scl", &self.last_scl)
            .field("last_sda", &self.last_sda)
            .field("slave_sda_pull", &self.slave_sda_pull)
            .field("attached", &self.addresses.len())
            .finish()
    }
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
            slave_sda_pull: false,
            tx_byte: 0,
            pending_ack: Ack::Nak,
            addresses: AddressMap::new(),
        }
    }

    /// Advance the state machine after the master writes either I2C
    /// line. `new_scl` / `new_sda` are the *effective* lines — the
    /// caller (state.rs::write_io) is expected to AND in the slave
    /// pull-down. The returned phase is observable through the public
    /// `phase` field.
    pub fn step(&mut self, new_scl: bool, new_sda: bool) {
        let prev_scl = self.last_scl;
        let prev_sda = self.last_sda;

        // START/STOP edges (SCL high throughout) take precedence over
        // ordinary SCL rise/fall sampling.
        if new_scl && prev_scl && prev_sda && !new_sda {
            self.handle_start();
        } else if new_scl && prev_scl && !prev_sda && new_sda {
            self.handle_stop();
        } else if new_scl && !prev_scl {
            self.on_scl_rise(new_sda);
        } else if !new_scl && prev_scl {
            self.on_scl_fall();
        }

        self.last_scl = new_scl;
        self.last_sda = new_sda;
    }

    fn handle_start(&mut self) {
        self.phase = I2cPhase::Started;
        self.current_target = None;
        self.last_byte = None;
        self.transactions = self.transactions.saturating_add(1);
        self.slave_sda_pull = false;
    }

    fn handle_stop(&mut self) {
        let target = self.current_target;
        self.phase = I2cPhase::Stopped;
        self.current_target = None;
        self.slave_sda_pull = false;
        if let Some(addr) = target
            && let Some(dev) = self.addresses.lookup(addr)
            && let Ok(mut d) = dev.lock()
        {
            d.on_stop();
        }
    }

    fn on_scl_rise(&mut self, sda: bool) {
        let sda_bit = sda as u8;
        match self.phase {
            I2cPhase::Idle => {
                // Spurious clock with no START — ignore.
            }
            I2cPhase::Started => {
                self.phase = I2cPhase::RxByte {
                    bits: sda_bit,
                    n: 1,
                };
            }
            I2cPhase::RxByte { bits, n } => {
                let new_bits = (bits << 1) | sda_bit;
                let new_n = n + 1;
                if new_n == 8 {
                    self.last_byte = Some(new_bits);
                    let was_addr = self.current_target.is_none();
                    if was_addr {
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
                    self.dispatch_byte_completion(was_addr, new_bits);
                } else {
                    self.phase = I2cPhase::RxByte {
                        bits: new_bits,
                        n: new_n,
                    };
                }
            }
            I2cPhase::AckMasterToSlave => {
                // 9th SCL rise — master is sampling the ACK on this
                // very edge. slave_sda_pull stays set through the whole
                // SCL-high period; it's released on the next SCL fall.
                self.phase = match self.current_dir {
                    I2cDir::Write => I2cPhase::RxByte { bits: 0, n: 0 },
                    I2cDir::Read => I2cPhase::TxByte { bits: 0, n: 0 },
                };
            }
            I2cPhase::TxByte { bits, n } => {
                // Master sampled bit on this rise — the bit was driven
                // on the previous fall via slave_sda_pull. Track what
                // was sent and advance.
                let bit_sent = (self.tx_byte >> 7) & 1;
                let new_bits = (bits << 1) | bit_sent;
                let new_n = n + 1;
                self.tx_byte <<= 1;
                if new_n == 8 {
                    self.last_byte = Some(new_bits);
                    self.phase = I2cPhase::AckSlaveToMaster;
                } else {
                    self.phase = I2cPhase::TxByte {
                        bits: new_bits,
                        n: new_n,
                    };
                }
            }
            I2cPhase::AckSlaveToMaster => {
                // Master drove ACK/NAK; sample sda. ACK = SDA low.
                let master_acked = !sda;
                let target = self.current_target.unwrap_or(0);
                if let Some(dev) = self.addresses.lookup(target)
                    && let Ok(mut d) = dev.lock()
                {
                    if master_acked {
                        d.on_master_ack();
                        self.tx_byte = d.on_read_byte();
                    } else {
                        d.on_master_nak();
                    }
                }
                // Continue with another byte slot — if the master
                // chose NAK, it will issue STOP/repeated START before
                // any further clocks shift this state.
                self.phase = I2cPhase::TxByte { bits: 0, n: 0 };
            }
            I2cPhase::Stopped => {
                self.phase = I2cPhase::Idle;
            }
        }
    }

    /// Compute slave_sda_pull for the upcoming SCL high period.
    /// Called on every SCL fall. The slave drives during the ACK clock
    /// (AckMasterToSlave) and during each TxByte bit; everywhere else
    /// it releases.
    fn on_scl_fall(&mut self) {
        match self.phase {
            I2cPhase::AckMasterToSlave => {
                self.slave_sda_pull = matches!(self.pending_ack, Ack::Ack);
            }
            I2cPhase::TxByte { .. } => {
                // Drive the current MSB of tx_byte. (Bit shift happens
                // on the rise after the master samples.)
                self.slave_sda_pull = (self.tx_byte >> 7) & 1 == 0;
            }
            _ => {
                self.slave_sda_pull = false;
            }
        }
    }

    /// Dispatch the byte that just completed in RxByte to whichever
    /// device is currently addressed. `was_addr` is true on the first
    /// byte of the transaction (the address byte itself); otherwise
    /// the byte is treated as a master-to-slave data write.
    fn dispatch_byte_completion(&mut self, was_addr: bool, byte: u8) {
        let target = match self.current_target {
            Some(a) => a,
            None => {
                self.pending_ack = Ack::Nak;
                return;
            }
        };
        let dev_opt = self.addresses.lookup(target);
        let Some(dev) = dev_opt else {
            self.pending_ack = Ack::Nak;
            return;
        };
        let Ok(mut d) = dev.lock() else {
            self.pending_ack = Ack::Nak;
            return;
        };
        if was_addr {
            d.on_start();
            self.pending_ack = Ack::Ack;
            if matches!(self.current_dir, I2cDir::Read) {
                self.tx_byte = d.on_read_byte();
            }
        } else {
            self.pending_ack = d.on_write_byte(byte);
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

    #[test]
    fn unaddressed_writes_naked() {
        // No device attached → slave_sda_pull stays false; an immediate
        // master read of SDA during the 9th high reads back high (NAK).
        let mut cpu = CpuState::new();
        start(&mut cpu);
        for i in (0..8).rev() {
            cpu.write_byte(IO_I2C_SDA, (0x94 >> i) & 1);
            cpu.write_byte(IO_I2C_SCL, 1);
            cpu.write_byte(IO_I2C_SCL, 0);
        }
        // Read the ACK bit from SDA during the 9th SCL high.
        cpu.write_byte(IO_I2C_SDA, 1);
        cpu.write_byte(IO_I2C_SCL, 1);
        let ack = cpu.read_byte(IO_I2C_SDA);
        assert_eq!(ack, 1, "no device attached should NAK");
    }

    /// Direct-bus harness: drive the I2C protocol without a CPU
    /// program. Used by both unit tests below and the integration test
    /// in tests/i2c.rs.
    fn bus_start(cpu: &mut CpuState) {
        cpu.write_byte(IO_I2C_SDA, 1);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SDA, 0);
        cpu.write_byte(IO_I2C_SCL, 0);
    }

    fn bus_stop(cpu: &mut CpuState) {
        cpu.write_byte(IO_I2C_SDA, 0);
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SDA, 1);
    }

    fn bus_write_byte(cpu: &mut CpuState, byte: u8) -> bool {
        for i in (0..8).rev() {
            cpu.write_byte(IO_I2C_SDA, (byte >> i) & 1);
            cpu.write_byte(IO_I2C_SCL, 1);
            cpu.write_byte(IO_I2C_SCL, 0);
        }
        cpu.write_byte(IO_I2C_SDA, 1);
        cpu.write_byte(IO_I2C_SCL, 1);
        let ack = cpu.read_byte(IO_I2C_SDA) == 0;
        cpu.write_byte(IO_I2C_SCL, 0);
        ack
    }

    fn bus_read_byte(cpu: &mut CpuState, master_acks: bool) -> u8 {
        let mut byte = 0u8;
        for _ in 0..8 {
            cpu.write_byte(IO_I2C_SDA, 1);
            cpu.write_byte(IO_I2C_SCL, 1);
            byte = (byte << 1) | (cpu.read_byte(IO_I2C_SDA) & 1);
            cpu.write_byte(IO_I2C_SCL, 0);
        }
        cpu.write_byte(IO_I2C_SDA, if master_acks { 0 } else { 1 });
        cpu.write_byte(IO_I2C_SCL, 1);
        cpu.write_byte(IO_I2C_SCL, 0);
        byte
    }

    #[test]
    fn add1_attached_acks_address_byte() {
        use crate::peripherals::i2c::devices::add1::Add1Device;
        use std::sync::{Arc, Mutex};

        let mut cpu = CpuState::new();
        let dev: Arc<Mutex<dyn crate::peripherals::i2c::device::I2cDevice>> =
            Arc::new(Mutex::new(Add1Device::new(0x50, 0x100)));
        cpu.io.i2c.addresses.insert(0x50, dev).unwrap();

        bus_start(&mut cpu);
        let acked = bus_write_byte(&mut cpu, 0x50 << 1);
        assert!(acked, "device at 0x50 should ACK its address byte");
    }

    #[test]
    fn add1_full_write_then_read_cycle() {
        use crate::peripherals::i2c::devices::add1::Add1Device;
        use std::sync::{Arc, Mutex};

        let mut cpu = CpuState::new();
        let dev: Arc<Mutex<dyn crate::peripherals::i2c::device::I2cDevice>> =
            Arc::new(Mutex::new(Add1Device::new(0x50, 0x100)));
        cpu.io.i2c.addresses.insert(0x50, dev).unwrap();

        // Write 0x42.
        bus_start(&mut cpu);
        assert!(bus_write_byte(&mut cpu, 0x50 << 1));
        assert!(bus_write_byte(&mut cpu, 0x42));
        bus_stop(&mut cpu);

        // Read two bytes — should be 0x43 then 0x44.
        bus_start(&mut cpu);
        assert!(bus_write_byte(&mut cpu, (0x50 << 1) | 1));
        let r1 = bus_read_byte(&mut cpu, true);
        let r2 = bus_read_byte(&mut cpu, false);
        bus_stop(&mut cpu);

        assert_eq!(r1, 0x43);
        assert_eq!(r2, 0x44);
    }

    #[test]
    fn unaddressed_address_naks_with_attached_device() {
        // Attach add1 at 0x50, address 0x42 (different) — NAK expected.
        use crate::peripherals::i2c::devices::add1::Add1Device;
        use std::sync::{Arc, Mutex};

        let mut cpu = CpuState::new();
        let dev: Arc<Mutex<dyn crate::peripherals::i2c::device::I2cDevice>> =
            Arc::new(Mutex::new(Add1Device::new(0x50, 0x100)));
        cpu.io.i2c.addresses.insert(0x50, dev).unwrap();

        bus_start(&mut cpu);
        let acked = bus_write_byte(&mut cpu, 0x42 << 1);
        assert!(!acked, "device at 0x50 should NOT ACK address 0x42");
    }
}

