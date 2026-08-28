//! `w25q32` — Winbond W25Q32 4 MiB serial NOR flash.
//!
//! Implements the standard-SPI instruction subset that a flash-
//! management demo needs: JEDEC ID, status register, write enable,
//! read data, page program, sector / block / chip erase. The image
//! is a `Vec<u8>` of exactly 4 MiB (0x40_0000 bytes) either loaded
//! from a host file (writes/erases propagate back on commit) or an
//! in-memory `0xFF` scratch (default flash state — erased).
//!
//! ## Faithful NOR semantics
//!
//! NOR flash can only flip individual bits **from 1 to 0** during a
//! Page Program; the only way to flip bits back from 0 to 1 is via
//! an erase (which sets a whole sector / block / chip to all `0xFF`).
//! We model this exactly: programming byte `B` over existing byte `E`
//! produces `E & B` (the AND rule). Writing `0x55` over `0xFF` works
//! (`0xFF & 0x55 == 0x55`); writing `0xAA` over `0x55` quietly does
//! `0x55 & 0xAA == 0x00`, not `0xAA`. Real silicon behaves the same.
//!
//! ## WIP / WEL timing
//!
//! Real W25Q32 sets the BUSY (= WIP) status bit while internal
//! program/erase operations run (~3 ms for Page Program, ~40 ms for
//! Sector Erase, ~200 ms for Block Erase, ~2 s for Chip Erase). We
//! don't `thread::sleep`; instead we count emulated byte-clocks via
//! `on_byte` calls and clear WIP when the counter reaches zero.
//! Chosen durations (in byte-clocks):
//!
//! | Operation     | Real ms | Byte-clocks |
//! |---------------|---------|-------------|
//! | Page Program  | ~3      | 1_024       |
//! | Sector Erase  | ~40     | 4_096       |
//! | Block Erase   | ~200    | 16_384      |
//! | Chip Erase    | ~2_000  | 65_536      |
//!
//! WEL (Write Enable Latch) is set by the `0x06` opcode and cleared
//! by `0x04` *or* automatically after each successful program/erase
//! (matches real-chip behavior — the master must re-enable before
//! every write).
//!
//! ## Out-of-scope (per brief 8 §Step 2)
//!
//! - **Dual / Quad SPI.** Standard single-SPI only.
//! - **4-byte addressing.** The 4 MiB capacity fits in 3 addr bytes.
//! - **CRC validation.** We don't model CRC at the wire level.
//! - **Page-wrap during program.** Real chips wrap within the
//!   256-byte page if the master sends more than 256 data bytes; we
//!   silently truncate at 256 to keep the state machine simple.

use std::collections::VecDeque;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::peripherals::spi::device::SpiDevice;

/// Total flash capacity in bytes (4 MiB).
pub const IMAGE_SIZE: usize = 4 * 1024 * 1024;
/// 256-byte page (the largest unit a single Page Program can write).
pub const PAGE_SIZE: usize = 256;
/// 4 KiB sector (smallest erase unit).
pub const SECTOR_SIZE: usize = 4 * 1024;
/// 64 KiB block (larger erase unit).
pub const BLOCK_SIZE_BYTES: usize = 64 * 1024;
/// Default CS pin per brief 8.
pub const DEFAULT_CS: u8 = 3;

/// Winbond W25Q32 signature: 0xEF (manufacturer) / 0x40 (memory type) /
/// 0x16 (capacity = 32 Mb = 4 MiB).
pub const JEDEC_ID: [u8; 3] = [0xEF, 0x40, 0x16];

const STATUS_WIP_BIT: u8 = 1 << 0;
const STATUS_WEL_BIT: u8 = 1 << 1;

const WIP_CLOCKS_PAGE_PROGRAM: u32 = 1_024;
const WIP_CLOCKS_SECTOR_ERASE: u32 = 4_096;
const WIP_CLOCKS_BLOCK_ERASE: u32 = 16_384;
const WIP_CLOCKS_CHIP_ERASE: u32 = 65_536;

/// Wire-level state for the in-progress transaction.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum WireState {
    /// No transaction in progress — next MOSI byte is an opcode.
    #[default]
    AwaitingOpcode,
    /// Collecting 3 address bytes for opcode (0x02, 0x03, 0x20, 0xD8).
    CollectingAddr {
        opcode: u8,
        addr: u32,
        bytes_done: u8,
    },
    /// Streaming data bytes from `image[cursor..]`. Wraps at IMAGE_SIZE.
    ReadingData { cursor: u32 },
    /// Repeatedly emitting the status register on every clock.
    ReadingStatus,
    /// Receiving Page Program data bytes. Bytes accumulate in
    /// `page_buf`; on CS deselect the page is committed (if WEL was
    /// set at the time of the program command).
    ProgrammingPage { base_addr: u32, authorized: bool },
}

pub struct W25q32Device {
    cs: u8,
    image: Vec<u8>,
    image_path: Option<PathBuf>,
    wel: bool,
    wip_clocks_remaining: u32,
    state: WireState,
    tx_buf: VecDeque<u8>,
    /// Buffer for in-flight Page Program data.
    page_buf: [u8; PAGE_SIZE],
    /// Number of data bytes received during the current Page Program.
    page_count: u16,
    last_accessed_address: Option<u32>,
}

impl Default for W25q32Device {
    fn default() -> Self {
        Self::new()
    }
}

impl W25q32Device {
    /// Fresh in-memory chip: 4 MiB of `0xFF`, default CS, no file.
    pub fn new() -> Self {
        Self::with_image(vec![0xFF; IMAGE_SIZE], None, DEFAULT_CS)
    }

    pub fn with_image(image: Vec<u8>, path: Option<PathBuf>, cs: u8) -> Self {
        // Normalize image length to exactly IMAGE_SIZE — pad with 0xFF
        // if short, truncate if long. Keeps the 3-byte address space
        // safe to index by `addr & 0x3FFFFF`.
        let mut image = image;
        if image.len() < IMAGE_SIZE {
            image.resize(IMAGE_SIZE, 0xFF);
        } else if image.len() > IMAGE_SIZE {
            image.truncate(IMAGE_SIZE);
        }
        Self {
            cs: cs & 0x7F,
            image,
            image_path: path,
            wel: false,
            wip_clocks_remaining: 0,
            state: WireState::AwaitingOpcode,
            tx_buf: VecDeque::new(),
            page_buf: [0xFF; PAGE_SIZE],
            page_count: 0,
            last_accessed_address: None,
        }
    }

    pub fn from_file(path: impl AsRef<Path>, cs: u8) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let image = fs::read(&path)?;
        Ok(Self::with_image(image, Some(path), cs))
    }

    // ----- Observers -----

    pub fn cs(&self) -> u8 {
        self.cs
    }

    pub fn image(&self) -> Vec<u8> {
        self.image.clone()
    }

    pub fn jedec_id(&self) -> [u8; 3] {
        JEDEC_ID
    }

    pub fn wip(&self) -> bool {
        self.wip_clocks_remaining > 0
    }

    pub fn wel(&self) -> bool {
        self.wel
    }

    pub fn last_accessed_address(&self) -> Option<u32> {
        self.last_accessed_address
    }

    /// Read a single byte from the image — useful for tests that
    /// want to assert post-program state without going through SPI.
    pub fn byte_at(&self, addr: u32) -> u8 {
        self.image[(addr as usize) & (IMAGE_SIZE - 1)]
    }

    // ----- Mutators (web-panel side, not the SPI bus) -----

    pub fn replace_image(&mut self, bytes: Vec<u8>) {
        self.image = bytes;
        if self.image.len() < IMAGE_SIZE {
            self.image.resize(IMAGE_SIZE, 0xFF);
        } else if self.image.len() > IMAGE_SIZE {
            self.image.truncate(IMAGE_SIZE);
        }
        self.sync_full_image_to_file();
    }

    pub fn erase_chip(&mut self) {
        for byte in self.image.iter_mut() {
            *byte = 0xFF;
        }
        self.wel = false;
        self.wip_clocks_remaining = WIP_CLOCKS_CHIP_ERASE;
        self.sync_full_image_to_file();
    }

    // ----- Internals -----

    fn status_byte(&self) -> u8 {
        let mut s = 0u8;
        if self.wip() {
            s |= STATUS_WIP_BIT;
        }
        if self.wel {
            s |= STATUS_WEL_BIT;
        }
        s
    }

    fn next_miso(&mut self) -> u8 {
        if let Some(b) = self.tx_buf.pop_front() {
            return b;
        }
        match self.state {
            WireState::ReadingData { cursor } => {
                let byte = self.image[(cursor as usize) & (IMAGE_SIZE - 1)];
                self.state = WireState::ReadingData {
                    cursor: cursor.wrapping_add(1),
                };
                byte
            }
            WireState::ReadingStatus => self.status_byte(),
            _ => 0xFF,
        }
    }

    fn handle_mosi(&mut self, mosi: u8) {
        // Tick WIP first — every byte clocked is a wall-time tick.
        if self.wip_clocks_remaining > 0 {
            self.wip_clocks_remaining -= 1;
        }

        match self.state {
            WireState::AwaitingOpcode => self.dispatch_opcode(mosi),
            WireState::CollectingAddr {
                opcode,
                addr,
                bytes_done,
            } => {
                let new_addr = (addr << 8) | (mosi as u32);
                let new_done = bytes_done + 1;
                if new_done < 3 {
                    self.state = WireState::CollectingAddr {
                        opcode,
                        addr: new_addr,
                        bytes_done: new_done,
                    };
                } else {
                    self.state = WireState::AwaitingOpcode;
                    self.dispatch_addressed(opcode, new_addr & 0x3FFFFF);
                }
            }
            WireState::ReadingData { .. } => {
                // Master clocking 0xFF; ignore MOSI.
            }
            WireState::ReadingStatus => {
                // Master clocking; ignore MOSI.
            }
            WireState::ProgrammingPage {
                base_addr,
                authorized,
            } => {
                if authorized && self.page_count < PAGE_SIZE as u16 {
                    self.page_buf[self.page_count as usize] = mosi;
                    self.page_count += 1;
                }
                // Truncation past 256 bytes is silent — real chips wrap
                // within the page; we just drop the extras.
                let _ = base_addr;
            }
        }
    }

    fn dispatch_opcode(&mut self, opcode: u8) {
        match opcode {
            0x9F => {
                // JEDEC ID: three response bytes (one-byte echo delay
                // means the first byte is emitted on the next exchange).
                for b in JEDEC_ID {
                    self.tx_buf.push_back(b);
                }
            }
            0x05 => {
                self.state = WireState::ReadingStatus;
            }
            0x06 => {
                self.wel = true;
            }
            0x04 => {
                self.wel = false;
            }
            0x03 | 0x02 | 0x20 | 0xD8 => {
                self.state = WireState::CollectingAddr {
                    opcode,
                    addr: 0,
                    bytes_done: 0,
                };
            }
            0xC7 | 0x60 if self.wel => {
                // Chip erase — no address bytes, requires WEL.
                for byte in self.image.iter_mut() {
                    *byte = 0xFF;
                }
                self.wel = false;
                self.wip_clocks_remaining = WIP_CLOCKS_CHIP_ERASE;
                self.sync_full_image_to_file();
            }
            _ => {
                // Unknown opcode: silently ignored, matching real-chip
                // lenient behaviour.
            }
        }
    }

    fn dispatch_addressed(&mut self, opcode: u8, addr: u32) {
        self.last_accessed_address = Some(addr);
        match opcode {
            0x03 => {
                // Read data: position cursor; subsequent on_byte calls
                // emit image[cursor], image[cursor+1], ... .
                self.state = WireState::ReadingData { cursor: addr };
            }
            0x02 => {
                // Page Program: enter receive mode. We snapshot WEL now
                // so a 0x04 mid-program doesn't retroactively cancel
                // (matches real-chip behaviour where the program is
                // committed on CS deselect regardless of WEL state at
                // that moment, provided WEL was set when the command
                // came in).
                self.page_count = 0;
                self.page_buf = [0xFF; PAGE_SIZE];
                self.state = WireState::ProgrammingPage {
                    base_addr: addr,
                    authorized: self.wel,
                };
            }
            0x20 if self.wel => {
                // Sector erase (4 KiB aligned), requires WEL.
                self.erase_range(addr & !((SECTOR_SIZE - 1) as u32), SECTOR_SIZE);
                self.wel = false;
                self.wip_clocks_remaining = WIP_CLOCKS_SECTOR_ERASE;
            }
            0xD8 if self.wel => {
                // Block erase (64 KiB aligned), requires WEL.
                self.erase_range(addr & !((BLOCK_SIZE_BYTES - 1) as u32), BLOCK_SIZE_BYTES);
                self.wel = false;
                self.wip_clocks_remaining = WIP_CLOCKS_BLOCK_ERASE;
            }
            _ => {}
        }
    }

    fn erase_range(&mut self, base: u32, size: usize) {
        let start = (base as usize) & (IMAGE_SIZE - 1);
        let end = (start + size).min(IMAGE_SIZE);
        for byte in &mut self.image[start..end] {
            *byte = 0xFF;
        }
        self.sync_range_to_file(start, end - start);
    }

    fn commit_page_program(&mut self, base_addr: u32) {
        if self.page_count == 0 {
            return;
        }
        let base = (base_addr as usize) & (IMAGE_SIZE - 1);
        // Pages don't span the 256-byte page boundary on real chips —
        // bytes past the boundary wrap. For us, just clip at IMAGE_SIZE
        // and let the state machine's 256-byte cap handle the rest.
        let n = (self.page_count as usize).min(PAGE_SIZE);
        for (i, &fresh) in self.page_buf[..n].iter().enumerate() {
            let addr = base + i;
            if addr >= IMAGE_SIZE {
                break;
            }
            // NOR semantics: only 1→0 transitions allowed without erase.
            self.image[addr] &= fresh;
        }
        self.sync_range_to_file(base, n.min(IMAGE_SIZE - base));
        self.wel = false;
        self.wip_clocks_remaining = WIP_CLOCKS_PAGE_PROGRAM;
    }

    fn sync_range_to_file(&self, offset: usize, len: usize) {
        if let Some(path) = self.image_path.as_ref()
            && let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(path)
            && f.seek(SeekFrom::Start(offset as u64)).is_ok()
        {
            let end = (offset + len).min(IMAGE_SIZE);
            let _ = f.write_all(&self.image[offset..end]);
        }
    }

    fn sync_full_image_to_file(&self) {
        if let Some(path) = self.image_path.as_ref() {
            let _ = fs::write(path, &self.image);
        }
    }
}

impl SpiDevice for W25q32Device {
    fn name(&self) -> &str {
        "w25q32"
    }

    fn on_select(&mut self) -> u8 {
        // Pre-load the first MISO byte. Drain anything already queued
        // (rare across CS edges, but cheap to support).
        self.next_miso()
    }

    fn on_byte(&mut self, mosi: u8) -> u8 {
        self.handle_mosi(mosi);
        self.next_miso()
    }

    fn on_deselect(&mut self) {
        // End-of-transaction cleanup. Commit any in-flight Page Program
        // here — real chips don't start the internal write until CS
        // rises. Reset wire state so the next CS-low transaction starts
        // from AwaitingOpcode.
        if let WireState::ProgrammingPage {
            base_addr,
            authorized,
        } = self.state
            && authorized
        {
            self.commit_page_program(base_addr);
        }
        self.state = WireState::AwaitingOpcode;
        self.page_count = 0;
        self.tx_buf.clear();
    }
}

/// Ergonomic extension on `SpiHandle<W25q32Device>` for the web panel.
pub trait W25q32HandleExt {
    fn image(&self) -> Vec<u8>;
    fn jedec_id(&self) -> [u8; 3];
    fn wip(&self) -> bool;
    fn wel(&self) -> bool;
    fn last_accessed_address(&self) -> Option<u32>;
    fn replace_image(&self, bytes: Vec<u8>);
    fn erase_chip(&self);
}

impl W25q32HandleExt for crate::peripherals::spi::SpiHandle<W25q32Device> {
    fn image(&self) -> Vec<u8> {
        self.with(|d| d.image())
    }
    fn jedec_id(&self) -> [u8; 3] {
        self.with(|d| d.jedec_id())
    }
    fn wip(&self) -> bool {
        self.with(|d| d.wip())
    }
    fn wel(&self) -> bool {
        self.with(|d| d.wel())
    }
    fn last_accessed_address(&self) -> Option<u32> {
        self.with(|d| d.last_accessed_address())
    }
    fn replace_image(&self, bytes: Vec<u8>) {
        self.with(|d| d.replace_image(bytes));
    }
    fn erase_chip(&self) {
        self.with(|d| d.erase_chip());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `bytes` through the device starting from `on_select`,
    /// returning the MISO bytes the master would have seen. The first
    /// MISO byte is from `on_select` (the byte for exchange 0); each
    /// subsequent byte is the return of `on_byte`.
    fn exchange(d: &mut W25q32Device, mosi: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(mosi.len());
        if mosi.is_empty() {
            return out;
        }
        // Exchange 0: master clocks mosi[0]; MISO is whatever on_select
        // returned (pre-loaded). on_byte(mosi[0]) decides what's on MISO
        // during exchange 1.
        let first = d.on_select();
        out.push(first);
        for (i, &b) in mosi.iter().enumerate() {
            let next_miso = d.on_byte(b);
            if i + 1 < mosi.len() {
                out.push(next_miso);
            }
        }
        out
    }

    #[test]
    fn name_and_default_cs() {
        let d = W25q32Device::new();
        assert_eq!(d.name(), "w25q32");
        assert_eq!(d.cs(), DEFAULT_CS);
        assert_eq!(d.image().len(), IMAGE_SIZE);
    }

    #[test]
    fn jedec_id_signature() {
        let mut d = W25q32Device::new();
        // 0x9F + 3 dummy bytes → response on exchanges 1..4 should be
        // [0xEF, 0x40, 0x16].
        let out = exchange(&mut d, &[0x9F, 0xFF, 0xFF, 0xFF]);
        // out[0] = on_select; out[1..4] = the three JEDEC bytes.
        assert_eq!(&out[1..4], &[0xEF, 0x40, 0x16]);
        assert_eq!(d.jedec_id(), [0xEF, 0x40, 0x16]);
    }

    #[test]
    fn read_uninitialized_returns_ff() {
        let mut d = W25q32Device::new();
        // 0x03 + 3 addr bytes + N data clocks. Fresh chip is all 0xFF.
        let mut req = vec![0x03, 0x00, 0x00, 0x00];
        req.extend(std::iter::repeat_n(0xFFu8, 8));
        let out = exchange(&mut d, &req);
        // The first 4 exchanges are opcode + 3 addr bytes; data starts
        // at exchange 4 (i.e., out[4..]).
        for &b in &out[4..] {
            assert_eq!(b, 0xFF);
        }
    }

    #[test]
    fn page_program_requires_wel() {
        let mut d = W25q32Device::new();
        // Try Page Program at addr 0 without 0x06 first.
        let mut req = vec![0x02, 0x00, 0x00, 0x00];
        req.extend(b"hello".iter().copied());
        // Drive command + data, then deselect to trigger commit.
        let _ = exchange(&mut d, &req);
        d.on_deselect();
        assert_eq!(d.byte_at(0), 0xFF, "no WEL = no program");
    }

    #[test]
    fn page_program_after_enable() {
        let mut d = W25q32Device::new();
        // Enable WEL.
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        assert!(d.wel());

        // Program 5 bytes at addr 0 (fresh 0xFF — full new value lands).
        let mut req = vec![0x02, 0x00, 0x00, 0x00];
        let payload = b"hello";
        req.extend(payload.iter().copied());
        let _ = exchange(&mut d, &req);
        d.on_deselect();

        for (i, &b) in payload.iter().enumerate() {
            assert_eq!(d.byte_at(i as u32), b);
        }
        // WEL auto-cleared on commit.
        assert!(!d.wel());
        // WIP set after commit.
        assert!(d.wip());
    }

    #[test]
    fn page_program_one_to_zero_rule() {
        let mut d = W25q32Device::new();
        // First write: 0x55 over 0xFF → 0x55 (every 1→0 allowed).
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let req: Vec<u8> = [0x02u8, 0x00, 0x00, 0x00, 0x55].to_vec();
        let _ = exchange(&mut d, &req);
        d.on_deselect();
        assert_eq!(d.byte_at(0), 0x55);

        // Second write: 0xAA over 0x55 (without erase). NOR can only
        // flip 1→0, so result = 0x55 & 0xAA = 0x00.
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let req: Vec<u8> = [0x02u8, 0x00, 0x00, 0x00, 0xAA].to_vec();
        let _ = exchange(&mut d, &req);
        d.on_deselect();
        assert_eq!(d.byte_at(0), 0x00, "1→0 AND rule");
    }

    #[test]
    fn sector_erase_clears_4kb() {
        let mut d = W25q32Device::new();
        // Fill 0..0x3000 with 0x00 — covers one sector below the
        // target (0..0x1000), the target itself (0x1000..0x2000), AND
        // one sector above (0x2000..0x3000) so the post-erase
        // not-touched assertions have something to check.
        for byte in &mut d.image[..0x3000] {
            *byte = 0x00;
        }
        // WEL + sector erase at addr 0x1234 (rounds down to 0x1000).
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let _ = exchange(&mut d, &[0x20, 0x00, 0x12, 0x34]);
        d.on_deselect();

        for i in 0..SECTOR_SIZE {
            assert_eq!(d.byte_at(0x1000 + i as u32), 0xFF);
        }
        // Bytes outside the erased sector are untouched.
        assert_eq!(d.byte_at(0x0FFF), 0x00);
        assert_eq!(d.byte_at(0x2000), 0x00);
        assert!(!d.wel(), "WEL auto-clears after erase");
    }

    #[test]
    fn block_erase_clears_64kb() {
        let mut d = W25q32Device::new();
        // Three blocks: 0..0x10000 (below), 0x10000..0x20000 (target),
        // 0x20000..0x30000 (above) — so the not-touched assertions on
        // 0x0FFFF and 0x20000 are valid.
        for byte in &mut d.image[..0x30000] {
            *byte = 0x00;
        }
        // Block erase at 0x12345 → block-aligned to 0x10000.
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let _ = exchange(&mut d, &[0xD8, 0x01, 0x23, 0x45]);
        d.on_deselect();

        for i in 0..BLOCK_SIZE_BYTES {
            assert_eq!(d.byte_at(0x10000 + i as u32), 0xFF);
        }
        assert_eq!(d.byte_at(0x0FFFF), 0x00);
        assert_eq!(d.byte_at(0x20000), 0x00);
    }

    #[test]
    fn chip_erase_clears_everything() {
        let mut d = W25q32Device::new();
        for byte in &mut d.image[..1024] {
            *byte = 0x00;
        }
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let _ = exchange(&mut d, &[0xC7]);
        d.on_deselect();
        for i in 0..1024 {
            assert_eq!(d.byte_at(i), 0xFF);
        }
        assert!(!d.wel());
    }

    #[test]
    fn wip_clears_after_clocks() {
        let mut d = W25q32Device::new();
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let req: Vec<u8> = [0x02u8, 0x00, 0x00, 0x00, 0x42].to_vec();
        let _ = exchange(&mut d, &req);
        d.on_deselect();
        assert!(d.wip(), "WIP set right after page-program commit");

        // Drive enough byte-clocks via Read Status to drain WIP.
        let mut poll = vec![0x05u8];
        poll.extend(std::iter::repeat_n(
            0xFFu8,
            WIP_CLOCKS_PAGE_PROGRAM as usize,
        ));
        let _ = exchange(&mut d, &poll);
        d.on_deselect();
        assert!(!d.wip(), "WIP must clear after the documented byte-clocks");
    }

    #[test]
    fn read_data_streams_image() {
        let mut d = W25q32Device::new();
        // Seed three bytes at addr 0x100.
        d.image[0x100] = 0x11;
        d.image[0x101] = 0x22;
        d.image[0x102] = 0x33;

        let mut req = vec![0x03, 0x00, 0x01, 0x00];
        req.extend(std::iter::repeat_n(0xFFu8, 3));
        let out = exchange(&mut d, &req);
        // out[0] = on_select; out[1..4] are responses to opcode + addr
        // bytes; data lands at out[4..7].
        assert_eq!(&out[4..7], &[0x11, 0x22, 0x33]);
        assert_eq!(d.last_accessed_address(), Some(0x100));
    }

    #[test]
    fn write_disable_clears_wel() {
        let mut d = W25q32Device::new();
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        assert!(d.wel());
        let _ = exchange(&mut d, &[0x04]);
        d.on_deselect();
        assert!(!d.wel());
    }

    #[test]
    fn image_persists_to_file() {
        let path =
            std::env::temp_dir().join(format!("cor24-w25q32-persist-{}.bin", std::process::id(),));
        // Write a fresh 4 MiB 0xFF image to disk first.
        fs::write(&path, vec![0xFFu8; IMAGE_SIZE]).expect("seed image");

        let mut d = W25q32Device::from_file(&path, DEFAULT_CS).expect("load");

        // Program a known pattern at addr 0x2000.
        let _ = exchange(&mut d, &[0x06]);
        d.on_deselect();
        let mut req = vec![0x02, 0x00, 0x20, 0x00];
        let payload: Vec<u8> = (0..32u8).collect();
        req.extend(payload.iter().copied());
        let _ = exchange(&mut d, &req);
        d.on_deselect();

        // Read the file back from disk and confirm.
        let on_disk = fs::read(&path).expect("read back image");
        assert_eq!(&on_disk[0x2000..0x2020], payload.as_slice());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn unknown_opcode_silently_ignored() {
        let mut d = W25q32Device::new();
        // 0xEE isn't in our table.
        let out = exchange(&mut d, &[0xEE, 0xFF, 0xFF]);
        // Just confirm nothing panicked; subsequent valid ops still work.
        let _ = out;
        let out = exchange(&mut d, &[0x9F, 0xFF, 0xFF, 0xFF]);
        assert_eq!(&out[1..4], &[0xEF, 0x40, 0x16]);
    }

    #[test]
    fn replace_image_clamps_to_4mib() {
        let mut d = W25q32Device::new();
        d.replace_image(vec![0x42; 100]);
        // Short input padded with 0xFF.
        assert_eq!(d.byte_at(0), 0x42);
        assert_eq!(d.byte_at(99), 0x42);
        assert_eq!(d.byte_at(100), 0xFF);
        assert_eq!(d.image().len(), IMAGE_SIZE);

        d.replace_image(vec![0xAA; IMAGE_SIZE + 1024]);
        // Long input truncated.
        assert_eq!(d.image().len(), IMAGE_SIZE);
        assert_eq!(d.byte_at((IMAGE_SIZE - 1) as u32), 0xAA);
    }
}
