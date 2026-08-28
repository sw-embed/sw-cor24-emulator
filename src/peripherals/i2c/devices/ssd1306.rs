//! `ssd1306` — Solomon Systech SSD1306 monochrome OLED controller (I2C).
//!
//! 128×64 (or 128×32) GDDRAM framebuffer behind the chip's two-byte-
//! transaction wire protocol: every write transaction starts with a
//! control byte whose top two bits tell us whether the rest of the
//! transaction is commands or pixel data, and whether the chip should
//! expect another control byte after a single payload byte. The web
//! panel pulls the framebuffer through [`Ssd1306HandleExt::framebuffer`]
//! and paints pale-green-on-black pixels.
//!
//! ## GDDRAM byte layout
//!
//! `framebuffer[page * 128 + col]` encodes 8 vertical pixels of column
//! `col` in page `page`, **LSB at the top**. This matches the chip's
//! wire format exactly — drivers can `memcpy` rows of pixel bytes into
//! us without rearranging.
//!
//! ## Control-byte protocol
//!
//! After START + 7-bit-address-write, the next byte is the control byte:
//!
//! | Co (bit 7) | D/C# (bit 6) | Effect                                         |
//! |------------|--------------|------------------------------------------------|
//! | 0          | 0            | Remaining bytes in the transaction are commands |
//! | 0          | 1            | Remaining bytes are GDDRAM data                  |
//! | 1          | 0            | Next byte is one command, then another control byte |
//! | 1          | 1            | Next byte is one data byte, then another control byte |
//!
//! Real drivers almost always pick `0x00` (init blocks) or `0x40`
//! (pixel blocks); the `Co=1` variants exist for one-shot mixed writes
//! and we honor them so contrived test drivers don't fault.
//!
//! ## Commands modeled
//!
//! Framebuffer-affecting commands (display on/off, addressing mode,
//! column/page ranges, page pointer, column-low/high nibble) execute
//! semantically. The rest of the init-sequence command space — contrast,
//! clock divide, multiplex, COM pins, charge pump, etc. — is
//! **lenient-consumed**: the parameter count is honored so the parser
//! stays aligned, but no state changes. Unknown opcodes log a warning
//! to stderr and consume one byte; subsequent bytes are treated as new
//! commands. This keeps demos running against drivers that send
//! commands we haven't modeled.
//!
//! ## Power-on default and persistence
//!
//! Fresh device: framebuffer all zero (every pixel off), display_on =
//! false (matches reset state — driver must send `0xAF` to enable),
//! addressing mode = Page (chip default), pointer at (0, 0).
//! Persistence is the consumer's concern; there's no battery on a real
//! SSD1306 either.

use crate::peripherals::i2c::device::{Ack, I2cDevice};

/// Default 7-bit address. Some boards strap to 0x3D via a solder bridge.
pub const DEFAULT_ADDRESS: u8 = 0x3C;

const PAGES: usize = 8;
const COLS: usize = 128;
const FRAMEBUFFER_LEN: usize = PAGES * COLS;

/// GDDRAM addressing modes (datasheet §10.1.3).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    /// Column auto-advances; on column wrap, page advances.
    Horizontal,
    /// Page auto-advances; on page wrap, column advances.
    Vertical,
    /// Column auto-advances within `column_start..=column_end`; page
    /// stays put until the master moves it. Chip default at reset.
    #[default]
    Page,
}

/// Tracks where in the per-transaction control-byte protocol we are.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum WireState {
    /// First byte of a new transaction — interpret as control byte.
    #[default]
    AwaitingControl,
    /// Co=0, D/C#=0: rest of transaction = command stream.
    Commanding,
    /// Co=0, D/C#=1: rest of transaction = GDDRAM data.
    Streaming,
    /// Co=1, D/C#=0: one command byte, then back to AwaitingControl.
    OneCommandLeft,
    /// Co=1, D/C#=1: one data byte, then back to AwaitingControl.
    OneDataLeft,
}

pub struct Ssd1306Device {
    address: u8,
    width: u16,
    height: u16,
    framebuffer: [u8; FRAMEBUFFER_LEN],
    display_on: bool,
    addressing_mode: AddressingMode,
    /// Current column write pointer (0..COLS).
    column: u8,
    /// Current page write pointer (0..PAGES).
    page: u8,
    /// Column range for horizontal/vertical modes (inclusive).
    column_start: u8,
    column_end: u8,
    /// Page range for horizontal/vertical modes (inclusive).
    page_start: u8,
    page_end: u8,
    wire_state: WireState,
    /// Opcode awaiting parameter bytes (None when not in a multi-byte
    /// command).
    command_pending: Option<u8>,
    /// Remaining parameter bytes to collect for `command_pending`.
    expecting_data_bytes: u8,
    command_buf: [u8; 3],
    buf_idx: u8,
}

impl Ssd1306Device {
    pub fn new(address: u8) -> Self {
        Self::with_size(address, 128, 64)
    }

    /// Build with explicit panel dimensions. Storage is always
    /// 128 × 8 pages internally; `height` just tells the web panel how
    /// many rows to render (32 = top 4 pages only). `width` accepts
    /// 128 today; other widths defer to a future brief.
    pub fn with_size(address: u8, width: u16, height: u16) -> Self {
        Self {
            address: address & 0x7F,
            width,
            height,
            framebuffer: [0; FRAMEBUFFER_LEN],
            display_on: false,
            addressing_mode: AddressingMode::Page,
            column: 0,
            page: 0,
            column_start: 0,
            column_end: (COLS - 1) as u8,
            page_start: 0,
            page_end: (PAGES - 1) as u8,
            wire_state: WireState::AwaitingControl,
            command_pending: None,
            expecting_data_bytes: 0,
            command_buf: [0; 3],
            buf_idx: 0,
        }
    }

    pub fn framebuffer(&self) -> [u8; FRAMEBUFFER_LEN] {
        self.framebuffer
    }

    pub fn display_on(&self) -> bool {
        self.display_on
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn addressing_mode(&self) -> AddressingMode {
        self.addressing_mode
    }

    pub fn column(&self) -> u8 {
        self.column
    }

    pub fn page(&self) -> u8 {
        self.page
    }

    fn handle_data_byte(&mut self, byte: u8) {
        let idx = (self.page as usize) * COLS + (self.column as usize);
        if idx < FRAMEBUFFER_LEN {
            self.framebuffer[idx] = byte;
        }
        self.advance_pointer();
    }

    fn advance_pointer(&mut self) {
        match self.addressing_mode {
            AddressingMode::Page => {
                // Column wraps within the column range; page does NOT
                // advance per the datasheet. Most drivers use this mode
                // because it's the chip default at reset.
                let next = self.column.wrapping_add(1);
                self.column = if next > self.column_end || (next as usize) >= COLS {
                    self.column_start
                } else {
                    next
                };
            }
            AddressingMode::Horizontal => {
                let next = self.column.wrapping_add(1);
                if next > self.column_end || (next as usize) >= COLS {
                    self.column = self.column_start;
                    let np = self.page.wrapping_add(1);
                    self.page = if np > self.page_end || (np as usize) >= PAGES {
                        self.page_start
                    } else {
                        np
                    };
                } else {
                    self.column = next;
                }
            }
            AddressingMode::Vertical => {
                let np = self.page.wrapping_add(1);
                if np > self.page_end || (np as usize) >= PAGES {
                    self.page = self.page_start;
                    let nc = self.column.wrapping_add(1);
                    self.column = if nc > self.column_end || (nc as usize) >= COLS {
                        self.column_start
                    } else {
                        nc
                    };
                } else {
                    self.page = np;
                }
            }
        }
    }

    fn handle_command_byte(&mut self, byte: u8) {
        if self.command_pending.is_some() {
            self.command_buf[self.buf_idx as usize] = byte;
            self.buf_idx += 1;
            if self.buf_idx >= self.expecting_data_bytes {
                let opcode = self.command_pending.take().unwrap();
                let params = self.command_buf;
                let len = self.buf_idx as usize;
                self.buf_idx = 0;
                self.expecting_data_bytes = 0;
                self.apply_command(opcode, &params[..len]);
            }
            return;
        }

        match command_param_count(byte) {
            Some(0) => self.apply_command(byte, &[]),
            Some(n) => {
                self.command_pending = Some(byte);
                self.expecting_data_bytes = n;
                self.buf_idx = 0;
            }
            None => {
                eprintln!("ssd1306: unknown opcode 0x{byte:02X} (consuming, no effect)");
            }
        }
    }

    fn apply_command(&mut self, opcode: u8, params: &[u8]) {
        match opcode {
            0xAE => self.display_on = false,
            0xAF => self.display_on = true,
            0x20 => {
                self.addressing_mode = match params[0] & 0x03 {
                    0 => AddressingMode::Horizontal,
                    1 => AddressingMode::Vertical,
                    _ => AddressingMode::Page, // 2 = page; 3 reserved → fall through
                };
            }
            0x21 => {
                self.column_start = params[0] & ((COLS - 1) as u8);
                self.column_end = params[1] & ((COLS - 1) as u8);
                self.column = self.column_start;
            }
            0x22 => {
                self.page_start = params[0] & ((PAGES - 1) as u8);
                self.page_end = params[1] & ((PAGES - 1) as u8);
                self.page = self.page_start;
            }
            0xB0..=0xB7 => self.page = opcode & 0x07,
            0x00..=0x0F => self.column = (self.column & 0xF0) | (opcode & 0x0F),
            0x10..=0x1F => self.column = (self.column & 0x0F) | ((opcode & 0x0F) << 4),
            // Lenient-consume: no state effect, parameter bytes already absorbed.
            _ => {}
        }
    }
}

/// Returns `Some(n)` for known opcodes where `n` is the number of
/// parameter bytes that follow the opcode. Returns `None` for opcodes
/// we don't recognise.
fn command_param_count(opcode: u8) -> Option<u8> {
    match opcode {
        // Framebuffer-affecting
        0xAE | 0xAF => Some(0),
        0x20 => Some(1),
        0x21 | 0x22 => Some(2),
        0xB0..=0xB7 => Some(0),
        0x00..=0x0F => Some(0),
        0x10..=0x1F => Some(0),
        // Lenient-consume
        0x81 => Some(1),        // contrast
        0x40..=0x7F => Some(0), // set display start line (0x40 | line)
        0xA0 | 0xA1 => Some(0), // segment remap
        0xA4 | 0xA5 => Some(0), // entire display on / resume
        0xA6 | 0xA7 => Some(0), // normal / inverse
        0xA8 => Some(1),        // multiplex ratio
        0xC0 | 0xC8 => Some(0), // COM scan direction
        0xD3 => Some(1),        // display offset
        0xD5 => Some(1),        // clock divide
        0xD9 => Some(1),        // precharge
        0xDA => Some(1),        // COM pins config
        0xDB => Some(1),        // vcomh deselect
        0x8D => Some(1),        // charge pump
        _ => None,
    }
}

impl I2cDevice for Ssd1306Device {
    fn address(&self) -> u8 {
        self.address
    }

    fn set_address(&mut self, addr: u8) {
        self.address = addr & 0x7F;
    }

    fn name(&self) -> &str {
        "ssd1306"
    }

    fn on_start(&mut self) {
        // Each transaction starts with a fresh control byte; carry the
        // command-pending state across STARTs only if a multi-byte
        // command's params haven't all arrived yet (matches real chip
        // behaviour — the bus state machine doesn't care about STARTs
        // within an open command).
        self.wire_state = WireState::AwaitingControl;
    }

    fn on_write_byte(&mut self, byte: u8) -> Ack {
        match self.wire_state {
            WireState::AwaitingControl => {
                let co = byte & 0x80 != 0;
                let dc = byte & 0x40 != 0;
                self.wire_state = match (co, dc) {
                    (false, false) => WireState::Commanding,
                    (false, true) => WireState::Streaming,
                    (true, false) => WireState::OneCommandLeft,
                    (true, true) => WireState::OneDataLeft,
                };
            }
            WireState::Commanding => self.handle_command_byte(byte),
            WireState::Streaming => self.handle_data_byte(byte),
            WireState::OneCommandLeft => {
                self.handle_command_byte(byte);
                self.wire_state = WireState::AwaitingControl;
            }
            WireState::OneDataLeft => {
                self.handle_data_byte(byte);
                self.wire_state = WireState::AwaitingControl;
            }
        }
        Ack::Ack
    }

    fn on_read_byte(&mut self) -> u8 {
        // The chip's status-register read (busy bit) is rarely used by
        // demos. Return 0 (not-busy) until a real demo needs more.
        0x00
    }
}

/// Ergonomic extension on `I2cHandle<Ssd1306Device>` for the web panel.
pub trait Ssd1306HandleExt {
    /// Snapshot the 1024-byte framebuffer. Called by the web panel each
    /// repaint tick.
    fn framebuffer(&self) -> [u8; FRAMEBUFFER_LEN];
    /// Display-on state. Panel renders blank when off.
    fn display_on(&self) -> bool;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

impl Ssd1306HandleExt for crate::peripherals::i2c::I2cHandle<Ssd1306Device> {
    fn framebuffer(&self) -> [u8; FRAMEBUFFER_LEN] {
        self.with(|d| d.framebuffer())
    }
    fn display_on(&self) -> bool {
        self.with(|d| d.display_on())
    }
    fn width(&self) -> u16 {
        self.with(|d| d.width())
    }
    fn height(&self) -> u16 {
        self.with(|d| d.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: drive a command-mode init burst (one START + 0x00
    /// control byte + opcodes).
    fn command_burst(d: &mut Ssd1306Device, bytes: &[u8]) {
        d.on_start();
        assert_eq!(d.on_write_byte(0x00), Ack::Ack); // control: commands
        for &b in bytes {
            assert_eq!(d.on_write_byte(b), Ack::Ack);
        }
    }

    /// Helper: drive a data-mode pixel burst (START + 0x40 + bytes).
    fn data_burst(d: &mut Ssd1306Device, bytes: &[u8]) {
        d.on_start();
        assert_eq!(d.on_write_byte(0x40), Ack::Ack); // control: data
        for &b in bytes {
            assert_eq!(d.on_write_byte(b), Ack::Ack);
        }
    }

    #[test]
    fn name_and_default_address() {
        let d = Ssd1306Device::new(DEFAULT_ADDRESS);
        assert_eq!(d.name(), "ssd1306");
        assert_eq!(d.address(), 0x3C);
        assert_eq!(d.width(), 128);
        assert_eq!(d.height(), 64);
    }

    #[test]
    fn framebuffer_starts_blank() {
        let d = Ssd1306Device::new(0x3C);
        assert_eq!(d.framebuffer(), [0u8; FRAMEBUFFER_LEN]);
        assert!(!d.display_on());
    }

    #[test]
    fn display_on_toggles() {
        let mut d = Ssd1306Device::new(0x3C);
        command_burst(&mut d, &[0xAF]);
        assert!(d.display_on());
        command_burst(&mut d, &[0xAE]);
        assert!(!d.display_on());
    }

    #[test]
    fn page_addressing_write_advances_column() {
        let mut d = Ssd1306Device::new(0x3C);
        // Default = Page mode. Write 3 bytes; column should be at 3.
        data_burst(&mut d, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(d.column(), 3);
        assert_eq!(d.page(), 0);
        assert_eq!(d.framebuffer()[0], 0xAA);
        assert_eq!(d.framebuffer()[1], 0xBB);
        assert_eq!(d.framebuffer()[2], 0xCC);
    }

    #[test]
    fn page_addressing_column_wraps() {
        let mut d = Ssd1306Device::new(0x3C);
        // 130 bytes — last two should land at cols 0 and 1 of page 0.
        let pixels: Vec<u8> = (0..130).map(|i| i as u8).collect();
        data_burst(&mut d, &pixels);
        // The wrap means cols 0 and 1 ended up with the LAST writes
        // (128 and 129), having been overwritten from the initial 0/1.
        assert_eq!(d.framebuffer()[0], 128);
        assert_eq!(d.framebuffer()[1], 129);
        assert_eq!(d.framebuffer()[127], 127);
        assert_eq!(d.column(), 2);
        assert_eq!(d.page(), 0); // Page mode never advances page.
    }

    #[test]
    fn horizontal_addressing_full_row() {
        let mut d = Ssd1306Device::new(0x3C);
        // Set mode = Horizontal, col 0..127, page 0..0.
        command_burst(&mut d, &[0x20, 0x00]);
        command_burst(&mut d, &[0x21, 0, (COLS - 1) as u8]);
        command_burst(&mut d, &[0x22, 0, 0]);

        let pixels: Vec<u8> = (0..128).map(|i| (i ^ 0x55) as u8).collect();
        data_burst(&mut d, &pixels);
        // Final write at col 127, page 0. Next write should wrap to col
        // 0, page 0 (since page range is [0..0]).
        assert_eq!(d.framebuffer()[0], 0x55); // 0 ^ 0x55
        assert_eq!(d.framebuffer()[127], (127u8) ^ 0x55);
        data_burst(&mut d, &[0xFF]);
        assert_eq!(d.framebuffer()[0], 0xFF); // wrapped back to col 0
    }

    #[test]
    fn horizontal_addressing_advances_page() {
        let mut d = Ssd1306Device::new(0x3C);
        command_burst(&mut d, &[0x20, 0x00]); // horizontal
        command_burst(&mut d, &[0x21, 0, (COLS - 1) as u8]);
        command_burst(&mut d, &[0x22, 0, 1]);

        // Write 256 bytes — first 128 land on page 0, next 128 on page 1.
        let pixels: Vec<u8> = (0..256).map(|i| i as u8).collect();
        data_burst(&mut d, &pixels);
        assert_eq!(d.framebuffer()[0], 0);
        assert_eq!(d.framebuffer()[127], 127);
        assert_eq!(d.framebuffer()[COLS], 128);
        assert_eq!(d.framebuffer()[COLS + 127], 255);
    }

    #[test]
    fn vertical_addressing_advances_page_first() {
        let mut d = Ssd1306Device::new(0x3C);
        command_burst(&mut d, &[0x20, 0x01]); // vertical
        command_burst(&mut d, &[0x21, 0, 1]); // col 0..1
        command_burst(&mut d, &[0x22, 0, 1]); // page 0..1

        // Vertical: page advances first. Order should be:
        // (col=0,page=0), (col=0,page=1), (col=1,page=0), (col=1,page=1).
        data_burst(&mut d, &[0xA0, 0xA1, 0xB0, 0xB1]);
        let fb = d.framebuffer();
        assert_eq!(fb[0], 0xA0); // page 0, col 0
        assert_eq!(fb[COLS], 0xA1); // page 1, col 0
        assert_eq!(fb[1], 0xB0); // page 0, col 1
        assert_eq!(fb[COLS + 1], 0xB1); // page 1, col 1
    }

    #[test]
    fn page_pointer_command_sets_page() {
        let mut d = Ssd1306Device::new(0x3C);
        command_burst(&mut d, &[0xB3]); // page = 3
        assert_eq!(d.page(), 3);
        data_burst(&mut d, &[0x77]);
        assert_eq!(d.framebuffer()[3 * COLS], 0x77);
    }

    #[test]
    fn column_low_and_high_nibble_compose_column_pointer() {
        let mut d = Ssd1306Device::new(0x3C);
        // Set col low nibble = 0x5, high nibble = 0x4 → col = 0x45.
        command_burst(&mut d, &[0x05, 0x14]);
        assert_eq!(d.column(), 0x45);
    }

    #[test]
    fn init_sequence_consumed() {
        // Standard Adafruit/Arduino-style init for 128x64. Ends with
        // 0xAF (display on). 0x2E (deactivate scroll) is not modeled
        // and should warn + skip without faulting.
        let mut d = Ssd1306Device::new(0x3C);
        let init = [
            0xAE, // display off
            0xD5, 0x80, // clock divide
            0xA8, 0x3F, // multiplex (64 rows)
            0xD3, 0x00, // display offset
            0x40, // start line = 0
            0x8D, 0x14, // charge pump on
            0x20, 0x00, // memory mode horizontal
            0xA1, // segment remap
            0xC8, // COM scan dec
            0xDA, 0x12, // COM pins
            0x81, 0xCF, // contrast
            0xD9, 0xF1, // precharge
            0xDB, 0x40, // vcomh
            0xA4, // entire display resume
            0xA6, // normal display
            0x2E, // deactivate scroll (unknown — should warn & skip)
            0xAF, // display on
        ];
        command_burst(&mut d, &init);
        assert!(d.display_on(), "post-init display should be on");
        assert_eq!(d.addressing_mode(), AddressingMode::Horizontal);
    }

    #[test]
    fn unknown_opcode_warns_but_does_not_panic() {
        let mut d = Ssd1306Device::new(0x3C);
        command_burst(&mut d, &[0x99]); // unknown
        // Subsequent pixel write should still work.
        data_burst(&mut d, &[0xDE, 0xAD]);
        assert_eq!(d.framebuffer()[0], 0xDE);
        assert_eq!(d.framebuffer()[1], 0xAD);
    }

    #[test]
    fn co1_mode_one_command_then_control_byte() {
        let mut d = Ssd1306Device::new(0x3C);
        // 0x80 + 0xAF = one command (display on), then another control byte.
        // Follow with 0x80 + 0xAE = display off.
        d.on_start();
        assert_eq!(d.on_write_byte(0x80), Ack::Ack);
        assert_eq!(d.on_write_byte(0xAF), Ack::Ack);
        assert!(d.display_on());
        assert_eq!(d.on_write_byte(0x80), Ack::Ack);
        assert_eq!(d.on_write_byte(0xAE), Ack::Ack);
        assert!(!d.display_on());
    }

    #[test]
    fn read_returns_zero() {
        let mut d = Ssd1306Device::new(0x3C);
        assert_eq!(d.on_read_byte(), 0x00);
        assert_eq!(d.on_read_byte(), 0x00);
    }

    #[test]
    fn set_address_masks_high_bit() {
        let mut d = Ssd1306Device::new(0x3C);
        d.set_address(0xFF);
        assert_eq!(d.address(), 0x7F);
    }

    #[test]
    fn with_size_32px_height_keeps_storage_at_full_8_pages() {
        // height=32 just narrows what the panel renders; internal
        // storage stays the full 1024 bytes (top 4 pages used; bottom
        // 4 pages ignored by the panel).
        let d = Ssd1306Device::with_size(0x3C, 128, 32);
        assert_eq!(d.width(), 128);
        assert_eq!(d.height(), 32);
        assert_eq!(d.framebuffer().len(), FRAMEBUFFER_LEN);
    }
}
