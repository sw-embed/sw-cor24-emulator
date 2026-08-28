//! `sdcard` — SD card in SPI mode.
//!
//! Models the canonical SD-SPI command subset that boot + read + write
//! demos need: CMD0/CMD8/CMD55+ACMD41 for the init handshake, CMD16
//! for blocklen, CMD17 for single-block read, CMD24 for single-block
//! write. The image is a `Vec<u8>` either loaded from a host file
//! (writes propagate back on each successful CMD24) or an in-memory
//! 1 MiB scratch of zeros.
//!
//! ## Wire protocol
//!
//! Every command is 6 bytes: `[0x40 | cmd_num, arg3, arg2, arg1, arg0,
//! crc]`. The slave responds with R1 (one byte; bit 7 clear = ready)
//! for most commands; CMD8 additionally emits 4 bytes of R7 echo.
//!
//! The master clocks `0xFF` bytes between transactions and during
//! response polling. The MOSI-byte top two bits being `01` (the high
//! command marker) is what wakes the slave from idle clocking into
//! command-collection mode.
//!
//! ## One-byte echo delay
//!
//! Per the `SpiDevice` trait contract, `on_byte(mosi)` returns the
//! byte the slave will drive on MISO during the *next* exchange. So
//! during the 6 cmd-byte exchanges, the slave drives 0xFF (idle); on
//! the 6th `on_byte` call (after all 6 bytes are collected and the
//! command is dispatched), the queued R1 byte is returned — landing
//! on the wire during the 7th exchange. That matches what real cards
//! do.
//!
//! ## Skip-on-this-pass (per brief 8 §Step 1)
//!
//! - **Pre-CMD0 dummy clocks.** Real cards need ≥74 clocks at idle
//!   before they accept CMD0; we accept immediately.
//! - **CRC validation on commands.** We accept any CRC byte from
//!   the master.
//! - **CRC computation on responses.** We emit two `0xFF` bytes as
//!   the trailing CRC after each data block; matches what
//!   software-only drivers expect (they don't check).
//! - **Multi-block read/write (CMD18/CMD25).** Single-block only.
//! - **SDHC vs SDSC distinction.** We present as SDHC always (the
//!   sector argument is interpreted as a 512-byte block index, never
//!   as a byte offset).
//!
//! ## Forward-compat: CS pin
//!
//! The registry spec form is `sdcard[@cs=<n>][?file=<path>]` per
//! brief 8. SPI is single-slave today (plan §9 future work is
//! multi-slave with a SELN bitmask); `@cs=<n>` is accepted by the
//! parser and stored for observation but not yet enforced — attaching
//! a second SPI device still replaces the first regardless of CS.
//! When multi-slave lands, existing `--spi-device sdcard@cs=2…` specs
//! will start routing correctly with no syntax change.

use std::collections::VecDeque;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::peripherals::spi::device::SpiDevice;

/// SPI block size SD/SDHC uses: 512 bytes per sector.
pub const BLOCK_SIZE: usize = 512;
/// Default in-memory scratch size when no `?file=` is provided.
pub const DEFAULT_IMAGE_SIZE: usize = 1024 * 1024;
/// Default chip-select pin per brief 8 (TMP125 sits on CS=1).
pub const DEFAULT_CS: u8 = 2;

/// Where in the command/response state machine the slave is. The bus
/// always thinks the master might start a new command, so
/// `AwaitingCommand` doubles as "idle, streaming queued response
/// bytes" — only `InCommand`, `AwaitingDataToken`, and `ReceivingData`
/// constrain the next MOSI byte's interpretation.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum WireState {
    /// No command in flight. If MOSI's top 2 bits are `01`, a new
    /// command begins; otherwise the byte is ignored (just clocking).
    #[default]
    AwaitingCommand,
    /// Collected `rx_n` bytes of the 6-byte command frame so far
    /// (1..=5). On the 6th byte the command is dispatched.
    InCommand { rx_n: u8 },
    /// CMD24 was accepted; we emitted R1=0x00 and are now waiting for
    /// the master to send the `0xFE` data token before the 512-byte
    /// payload. Bytes other than `0xFE` are wait-clocks.
    AwaitingDataToken { sector: u32 },
    /// Master sent `0xFE`; we're collecting 512 data bytes followed
    /// by 2 CRC bytes (`count` runs 0..514). On count==514 the block
    /// is committed to `image` (and the host file if backed).
    ReceivingData { sector: u32, count: u16 },
}

pub struct SdCardDevice {
    /// In-memory bytes-eye view of the disk image. With no host file,
    /// initialized to `DEFAULT_IMAGE_SIZE` zeros.
    image: Vec<u8>,
    /// Optional backing file. When set, every successful CMD24 writes
    /// the new sector contents back to disk.
    image_path: Option<PathBuf>,
    state: WireState,
    /// The 6-byte command-collection buffer.
    rx_buf: [u8; 6],
    /// Queue of bytes the slave will drive on MISO across future
    /// exchanges. Popped one byte per exchange in `next_miso`.
    tx_buf: VecDeque<u8>,
    /// Accumulating buffer for CMD24's incoming 512+2 byte payload.
    write_buf: Vec<u8>,
    /// CMD55 just dispatched — the next command should be interpreted
    /// as ACMD<num>. Cleared after any command is processed.
    acmd_pending: bool,
    /// Most recently CMD17- or CMD24-accessed sector, exposed through
    /// the HandleExt for the web panel's "currently reading X"
    /// indicator.
    last_accessed_sector: Option<u32>,
    /// Configured CS pin. Stored for observation; single-slave bus
    /// ignores it.
    cs: u8,
}

impl Default for SdCardDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl SdCardDevice {
    /// Empty in-memory 1 MiB scratch, default CS.
    pub fn new() -> Self {
        Self::with_image(vec![0; DEFAULT_IMAGE_SIZE], None, DEFAULT_CS)
    }

    pub fn with_image(image: Vec<u8>, path: Option<PathBuf>, cs: u8) -> Self {
        Self {
            image,
            image_path: path,
            state: WireState::AwaitingCommand,
            rx_buf: [0; 6],
            tx_buf: VecDeque::new(),
            write_buf: Vec::with_capacity(BLOCK_SIZE + 2),
            acmd_pending: false,
            last_accessed_sector: None,
            cs,
        }
    }

    /// Load image bytes from the given path. The path is remembered so
    /// later writes propagate back.
    pub fn from_file(path: impl AsRef<Path>, cs: u8) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let image = fs::read(&path)?;
        Ok(Self::with_image(image, Some(path), cs))
    }

    pub fn image(&self) -> Vec<u8> {
        self.image.clone()
    }

    pub fn size(&self) -> usize {
        self.image.len()
    }

    pub fn last_accessed_sector(&self) -> Option<u32> {
        self.last_accessed_sector
    }

    pub fn replace_image(&mut self, bytes: Vec<u8>) {
        self.image = bytes;
    }

    pub fn cs(&self) -> u8 {
        self.cs
    }

    fn next_miso(&mut self) -> u8 {
        self.tx_buf.pop_front().unwrap_or(0xFF)
    }

    fn handle_mosi(&mut self, mosi: u8) {
        match self.state {
            WireState::AwaitingCommand => {
                // Top 2 bits == 01 marks an SD command opcode.
                if (mosi & 0xC0) == 0x40 {
                    self.rx_buf[0] = mosi;
                    self.state = WireState::InCommand { rx_n: 1 };
                }
                // Otherwise: master is clocking 0xFF (or noise). No state
                // change; the queued response continues to drain.
            }
            WireState::InCommand { rx_n } => {
                self.rx_buf[rx_n as usize] = mosi;
                if rx_n == 5 {
                    let opcode = self.rx_buf[0];
                    let arg = [
                        self.rx_buf[1],
                        self.rx_buf[2],
                        self.rx_buf[3],
                        self.rx_buf[4],
                    ];
                    self.state = WireState::AwaitingCommand;
                    self.handle_command(opcode, arg);
                } else {
                    self.state = WireState::InCommand { rx_n: rx_n + 1 };
                }
            }
            WireState::AwaitingDataToken { sector } => {
                if mosi == 0xFE {
                    self.write_buf.clear();
                    self.state = WireState::ReceivingData { sector, count: 0 };
                }
                // Other bytes are wait clocks; ignore.
            }
            WireState::ReceivingData { sector, count } => {
                self.write_buf.push(mosi);
                let new_count = count + 1;
                if new_count == (BLOCK_SIZE + 2) as u16 {
                    self.state = WireState::AwaitingCommand;
                    self.finalize_write(sector);
                } else {
                    self.state = WireState::ReceivingData {
                        sector,
                        count: new_count,
                    };
                }
            }
        }
    }

    fn handle_command(&mut self, opcode: u8, arg: [u8; 4]) {
        let cmd_num = opcode & 0x3F;
        let is_acmd = self.acmd_pending;
        self.acmd_pending = false;

        match (is_acmd, cmd_num) {
            // CMD0: GO_IDLE_STATE → R1 = 0x01 (idle).
            (false, 0) => self.tx_buf.push_back(0x01),
            // CMD8: SEND_IF_COND → R1 + 4-byte voltage echo.
            (false, 8) => {
                self.tx_buf.push_back(0x01);
                self.tx_buf.push_back(0x00);
                self.tx_buf.push_back(0x00);
                self.tx_buf.push_back(0x01);
                self.tx_buf.push_back(0xAA);
            }
            // CMD55: APP_CMD prefix → R1 = 0x01; arm acmd_pending.
            (false, 55) => {
                self.tx_buf.push_back(0x01);
                self.acmd_pending = true;
            }
            // ACMD41: SD_SEND_OP_COND → R1 = 0x00 (ready).
            (true, 41) => self.tx_buf.push_back(0x00),
            // CMD16: SET_BLOCKLEN → R1 = 0x00 (we hard-code 512).
            (false, 16) => self.tx_buf.push_back(0x00),
            // CMD17: READ_SINGLE_BLOCK.
            (false, 17) => {
                let sector = u32::from_be_bytes(arg);
                self.read_block(sector);
            }
            // CMD24: WRITE_BLOCK — R1=0x00, then await data token.
            (false, 24) => {
                let sector = u32::from_be_bytes(arg);
                self.tx_buf.push_back(0x00);
                self.last_accessed_sector = Some(sector);
                self.state = WireState::AwaitingDataToken { sector };
            }
            // Anything else: R1 with illegal-command bit set.
            _ => self.tx_buf.push_back(0x04),
        }
    }

    fn read_block(&mut self, sector: u32) {
        self.last_accessed_sector = Some(sector);
        let offset = (sector as usize) * BLOCK_SIZE;
        // R1 = 0x00 (success).
        self.tx_buf.push_back(0x00);
        // 8 busy bytes — matches "Several 0xFF bytes" from the brief.
        for _ in 0..8 {
            self.tx_buf.push_back(0xFF);
        }
        // Data token.
        self.tx_buf.push_back(0xFE);
        // 512 data bytes (zero-fill if sector is past image end).
        for i in 0..BLOCK_SIZE {
            self.tx_buf
                .push_back(self.image.get(offset + i).copied().unwrap_or(0));
        }
        // Two dummy CRC bytes — software-only drivers don't check.
        self.tx_buf.push_back(0xFF);
        self.tx_buf.push_back(0xFF);
    }

    fn finalize_write(&mut self, sector: u32) {
        let offset = (sector as usize) * BLOCK_SIZE;
        let need = offset + BLOCK_SIZE;
        if self.image.len() < need {
            self.image.resize(need, 0);
        }
        // First 512 bytes of write_buf are data; the trailing 2 are CRC.
        self.image[offset..offset + BLOCK_SIZE].copy_from_slice(&self.write_buf[..BLOCK_SIZE]);

        // Propagate to the host file if backed. We seek+write the
        // single sector rather than rewriting the whole image — keeps
        // large images responsive. `truncate(false)` is explicit so
        // we preserve other sectors when reopening an existing image.
        if let Some(path) = self.image_path.clone()
            && let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)
            && f.seek(SeekFrom::Start(offset as u64)).is_ok()
        {
            let _ = f.write_all(&self.image[offset..offset + BLOCK_SIZE]);
        }

        // Data-accepted response — drives MISO after the CRC bytes.
        self.tx_buf.push_back(0xE5);
    }
}

impl SpiDevice for SdCardDevice {
    fn name(&self) -> &str {
        "sdcard"
    }

    fn on_select(&mut self) -> u8 {
        // Pre-load the first MISO byte. With no pending response,
        // drive 0xFF (idle), matching what a real card does between
        // commands.
        self.next_miso()
    }

    fn on_byte(&mut self, mosi: u8) -> u8 {
        self.handle_mosi(mosi);
        self.next_miso()
    }

    fn on_deselect(&mut self) {
        // Don't reset state — drivers commonly toggle CS between the
        // 6-byte command and the response poll. Wire/tx state survives.
    }
}

/// Ergonomic extension on `SpiHandle<SdCardDevice>` for the web panel.
pub trait SdCardHandleExt {
    fn image(&self) -> Vec<u8>;
    fn size(&self) -> usize;
    fn last_accessed_sector(&self) -> Option<u32>;
    fn replace_image(&self, bytes: Vec<u8>);
}

impl SdCardHandleExt for crate::peripherals::spi::SpiHandle<SdCardDevice> {
    fn image(&self) -> Vec<u8> {
        self.with(|d| d.image())
    }
    fn size(&self) -> usize {
        self.with(|d| d.size())
    }
    fn last_accessed_sector(&self) -> Option<u32> {
        self.with(|d| d.last_accessed_sector())
    }
    fn replace_image(&self, bytes: Vec<u8>) {
        self.with(|d| d.replace_image(bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a 6-byte SD command. Returns the MISO byte the slave
    /// drives during the *next* exchange after the 6th cmd byte —
    /// that's where R1 lands.
    fn send_cmd(d: &mut SdCardDevice, cmd_num: u8, arg: u32) -> u8 {
        let opcode = 0x40 | (cmd_num & 0x3F);
        let bytes = [
            opcode,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            0xFF, // CRC placeholder; we accept any value.
        ];
        let _ = d.on_select();
        let mut last = 0xFF;
        for &b in &bytes {
            last = d.on_byte(b);
        }
        last
    }

    #[test]
    fn name_and_default_cs() {
        let d = SdCardDevice::new();
        assert_eq!(d.name(), "sdcard");
        assert_eq!(d.cs(), DEFAULT_CS);
        assert_eq!(d.size(), DEFAULT_IMAGE_SIZE);
    }

    #[test]
    fn cmd0_returns_idle() {
        let mut d = SdCardDevice::new();
        let r1 = send_cmd(&mut d, 0, 0);
        assert_eq!(r1, 0x01, "CMD0 should return R1=0x01 (idle)");
    }

    #[test]
    fn cmd8_echo() {
        let mut d = SdCardDevice::new();
        let r1 = send_cmd(&mut d, 8, 0x000001AA);
        assert_eq!(r1, 0x01, "CMD8 R1 should be 0x01 (idle, supported)");
        // The 4-byte R7 echo follows on the next 4 clocks.
        let echo: Vec<u8> = (0..4).map(|_| d.on_byte(0xFF)).collect();
        assert_eq!(echo, vec![0x00, 0x00, 0x01, 0xAA]);
    }

    #[test]
    fn acmd41_transitions_to_ready() {
        let mut d = SdCardDevice::new();
        // CMD55 arms the ACMD prefix.
        let r1 = send_cmd(&mut d, 55, 0);
        assert_eq!(r1, 0x01);
        // ACMD41 should return R1=0x00 (ready).
        let r1 = send_cmd(&mut d, 41, 0);
        assert_eq!(r1, 0x00, "ACMD41 should return ready (0x00)");
    }

    #[test]
    fn cmd16_acceptance() {
        let mut d = SdCardDevice::new();
        let r1 = send_cmd(&mut d, 16, 512);
        assert_eq!(r1, 0x00);
    }

    #[test]
    fn cmd17_reads_known_sector() {
        let mut d = SdCardDevice::new();
        // Preload sector 0 with a known pattern.
        let pattern: Vec<u8> = (0..BLOCK_SIZE).map(|i| (i & 0xFF) as u8).collect();
        d.image[..BLOCK_SIZE].copy_from_slice(&pattern);

        let r1 = send_cmd(&mut d, 17, 0);
        assert_eq!(r1, 0x00, "CMD17 R1 should be 0x00");

        // Skip past the busy bytes until the data token.
        let token = poll_for_response_token(&mut d, 32).expect("data token");
        assert_eq!(token, 0xFE, "expected data token");

        // Read 512 data bytes.
        let data: Vec<u8> = (0..BLOCK_SIZE).map(|_| d.on_byte(0xFF)).collect();
        assert_eq!(data, pattern, "CMD17 data must match preloaded sector");

        // Trailing 2 CRC bytes (dummy 0xFF).
        let crc1 = d.on_byte(0xFF);
        let crc2 = d.on_byte(0xFF);
        assert_eq!((crc1, crc2), (0xFF, 0xFF));
        assert_eq!(d.last_accessed_sector(), Some(0));
    }

    /// Poll until `0xFE` (the SD data token) appears.
    fn poll_for_response_token(d: &mut SdCardDevice, max: usize) -> Option<u8> {
        for _ in 0..max {
            let b = d.on_byte(0xFF);
            if b == 0xFE {
                return Some(b);
            }
        }
        None
    }

    #[test]
    fn cmd17_reads_sector_five() {
        let mut d = SdCardDevice::new();
        let pattern: Vec<u8> = (0..BLOCK_SIZE).map(|i| ((i ^ 0x55) & 0xFF) as u8).collect();
        let offset = 5 * BLOCK_SIZE;
        d.image[offset..offset + BLOCK_SIZE].copy_from_slice(&pattern);

        assert_eq!(send_cmd(&mut d, 17, 5), 0x00);
        assert_eq!(poll_for_response_token(&mut d, 32), Some(0xFE));
        let data: Vec<u8> = (0..BLOCK_SIZE).map(|_| d.on_byte(0xFF)).collect();
        assert_eq!(data, pattern);
    }

    #[test]
    fn cmd24_writes_persists() {
        let mut d = SdCardDevice::new();
        // Init handshake (not strictly required for this device, but
        // exercises the path drivers take in practice).
        assert_eq!(send_cmd(&mut d, 0, 0), 0x01);
        assert_eq!(send_cmd(&mut d, 8, 0x000001AA), 0x01);
        let _r7: Vec<u8> = (0..4).map(|_| d.on_byte(0xFF)).collect();
        assert_eq!(send_cmd(&mut d, 55, 0), 0x01);
        assert_eq!(send_cmd(&mut d, 41, 0), 0x00);

        // Write sector 5 with a fresh pattern.
        let pattern: Vec<u8> = (0..BLOCK_SIZE).map(|i| 0xA5u8 ^ (i as u8)).collect();
        assert_eq!(send_cmd(&mut d, 24, 5), 0x00);
        // Send the data-token 0xFE.
        let _ = d.on_byte(0xFE);
        // 512 data bytes.
        for &b in &pattern {
            let _ = d.on_byte(b);
        }
        // 2 dummy CRC bytes — the 2nd one is the cycle that finalizes
        // the write; after it the slave drives 0xE5 (data accepted).
        let _ = d.on_byte(0xFF);
        let accepted = d.on_byte(0xFF);
        assert_eq!(accepted, 0xE5, "expected data-accepted response");

        // Read it back via CMD17.
        assert_eq!(send_cmd(&mut d, 17, 5), 0x00);
        assert_eq!(poll_for_response_token(&mut d, 32), Some(0xFE));
        let readback: Vec<u8> = (0..BLOCK_SIZE).map(|_| d.on_byte(0xFF)).collect();
        assert_eq!(readback, pattern, "CMD24 must persist for CMD17");
    }

    #[test]
    fn cmd24_writes_to_file() {
        // Create a tempfile-backed image. We use std::env::temp_dir +
        // a deterministic per-test name; cleanup is on a best-effort
        // basis in the success path.
        let path = std::env::temp_dir().join(format!(
            "cor24-sdcard-cmd24-test-{}.img",
            std::process::id(),
        ));
        // 6 sectors of zeros up front so sector 3 is in range.
        fs::write(&path, vec![0u8; 6 * BLOCK_SIZE]).expect("seed image");

        let mut d = SdCardDevice::from_file(&path, DEFAULT_CS).expect("load");

        let pattern: Vec<u8> = (0..BLOCK_SIZE)
            .map(|i| (i.wrapping_mul(7) & 0xFF) as u8)
            .collect();
        assert_eq!(send_cmd(&mut d, 24, 3), 0x00);
        let _ = d.on_byte(0xFE);
        for &b in &pattern {
            let _ = d.on_byte(b);
        }
        let _ = d.on_byte(0xFF);
        let accepted = d.on_byte(0xFF);
        assert_eq!(accepted, 0xE5);

        // Verify the host file now reflects the new sector contents.
        let on_disk = fs::read(&path).expect("read back image");
        let offset = 3 * BLOCK_SIZE;
        assert_eq!(&on_disk[offset..offset + BLOCK_SIZE], pattern.as_slice());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn unknown_command_returns_illegal_bit() {
        let mut d = SdCardDevice::new();
        // CMD63 isn't in our table.
        let r1 = send_cmd(&mut d, 63, 0);
        assert_eq!(r1, 0x04, "unknown cmd should set illegal-command bit");
    }

    #[test]
    fn idle_clocks_return_ff_without_pending_response() {
        let mut d = SdCardDevice::new();
        for _ in 0..32 {
            assert_eq!(d.on_byte(0xFF), 0xFF);
        }
    }

    #[test]
    fn replace_image_round_trip() {
        let mut d = SdCardDevice::new();
        let blob = vec![0xAB; BLOCK_SIZE];
        d.replace_image(blob.clone());
        assert_eq!(d.image(), blob);
        assert_eq!(d.size(), BLOCK_SIZE);
    }
}
