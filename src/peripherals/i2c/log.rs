//! Chronological log of I2C bus events.
//!
//! Mirrors the shape of `UartLog`: pure data, written to by the bus
//! state machine on every START/STOP/byte completion, read by the CLI
//! (`--dump-i2c`) and the future Web UI to render a transaction view.
//!
//! Entries are not serialized — they're runtime/transport state and
//! can grow large during long bus runs.

use serde::{Deserialize, Serialize};

use crate::cpu::i2c_bus::I2cDir;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum I2cEvent {
    Start,
    Stop,
    /// Address byte: `addr` is 7-bit; `ack` reflects the slave's
    /// response (false = NAK, no device at this address).
    Address { addr: u8, dir: I2cDir, ack: bool },
    /// Master-to-slave data byte. `ack` is the slave's response.
    WriteByte { addr: u8, byte: u8, ack: bool },
    /// Slave-to-master data byte. The master's ACK/NAK following
    /// the read is logged as a separate event when relevant.
    ReadByte { addr: u8, byte: u8 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct I2cLogEntry {
    pub event: I2cEvent,
    pub instruction: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct I2cLog {
    #[serde(skip)]
    entries: Vec<I2cLogEntry>,
}

impl I2cLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: I2cEvent, instruction: u64) {
        self.entries.push(I2cLogEntry { event, instruction });
    }

    pub fn entries(&self) -> &[I2cLogEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Render the log as a UartLog-shaped multiline string. Each line
    /// is prefixed with `I2C:` and a left-padded instruction number,
    /// then a fixed-width event tag, then event-specific fields.
    pub fn format(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&format_entry(entry));
            out.push('\n');
        }
        out
    }
}

fn format_entry(entry: &I2cLogEntry) -> String {
    let prefix = format!("  I2C: i={:>9}  ", entry.instruction);
    match &entry.event {
        I2cEvent::Start => format!("{prefix}START"),
        I2cEvent::Stop => format!("{prefix}STOP"),
        I2cEvent::Address { addr, dir, ack } => {
            let dir = match dir {
                I2cDir::Write => "WR",
                I2cDir::Read => "RD",
            };
            let ack = if *ack { "ACK" } else { "NAK" };
            format!("{prefix}ADDR 0x{:02X} {dir} {ack}", addr)
        }
        I2cEvent::WriteByte { addr, byte, ack } => {
            let ack = if *ack { "ACK" } else { "NAK" };
            format!("{prefix}WR   0x{:02X} 0x{:02X} {ack}", addr, byte)
        }
        I2cEvent::ReadByte { addr, byte } => {
            format!("{prefix}RD   0x{:02X} 0x{:02X}", addr, byte)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_formats_as_empty_string() {
        let log = I2cLog::new();
        assert!(log.is_empty());
        assert_eq!(log.format(), "");
    }

    #[test]
    fn entries_logged_in_order() {
        let mut log = I2cLog::new();
        log.push(I2cEvent::Start, 100);
        log.push(
            I2cEvent::Address {
                addr: 0x4A,
                dir: I2cDir::Write,
                ack: true,
            },
            120,
        );
        log.push(
            I2cEvent::WriteByte {
                addr: 0x4A,
                byte: 0x01,
                ack: true,
            },
            150,
        );
        log.push(I2cEvent::Stop, 200);
        assert_eq!(log.len(), 4);
        assert_eq!(log.entries()[0].instruction, 100);
        assert_eq!(log.entries()[3].event, I2cEvent::Stop);
    }

    #[test]
    fn format_shape_matches_spec() {
        let mut log = I2cLog::new();
        log.push(I2cEvent::Start, 1);
        log.push(
            I2cEvent::Address {
                addr: 0x4A,
                dir: I2cDir::Read,
                ack: true,
            },
            2,
        );
        log.push(
            I2cEvent::ReadByte {
                addr: 0x4A,
                byte: 0x19,
            },
            3,
        );
        log.push(I2cEvent::Stop, 4);

        let s = log.format();
        assert!(s.contains("START"));
        assert!(s.contains("ADDR 0x4A RD ACK"));
        assert!(s.contains("RD   0x4A 0x19"));
        assert!(s.contains("STOP"));
        // 4 lines, all prefixed with "  I2C: ".
        assert_eq!(s.lines().count(), 4);
        assert!(s.lines().all(|l| l.starts_with("  I2C: ")));
    }

    #[test]
    fn format_shows_nak_for_unaddressed() {
        let mut log = I2cLog::new();
        log.push(
            I2cEvent::Address {
                addr: 0x55,
                dir: I2cDir::Write,
                ack: false,
            },
            1,
        );
        let s = log.format();
        assert!(s.contains("ADDR 0x55 WR NAK"), "got: {s}");
    }

    #[test]
    fn clear_empties_log() {
        let mut log = I2cLog::new();
        log.push(I2cEvent::Start, 1);
        log.push(I2cEvent::Stop, 2);
        assert_eq!(log.len(), 2);
        log.clear();
        assert!(log.is_empty());
    }
}
