Phase C.3 — SPI shift-register state machine.

Mirror of I2C step 004 i2c-bus-state-machine, but much simpler: no addressing, no START/STOP, no open-drain. Just a shift register clocked by SCLK while SELN is low.

Plan ref: docs/feature-i2c-spi-emu.md sec 4.6.

What to do:
1. New module src/cpu/spi_bus.rs (sibling of src/cpu/i2c_bus.rs):
   - SpiBusState struct: sclk_last (for edge detection), bit_count (0..=7), shift_in (u8 — accumulating from MOSI), shift_out (u8 — staged for MISO; for now stays 0 since no device), bytes_exchanged (u32 — observability counter), last_mosi_byte (Option<u8>), last_miso_byte (Option<u8>).
   - SpiBusState::new() / Default.
   - SpiBusState::step(sclk: bool, mosi_bit: bool, seln: bool, instruction: u64): on rising edge of SCLK with seln == false, shift_in = (shift_in << 1) | mosi_bit; bit_count++; if bit_count == 8: last_mosi_byte = Some(shift_in); last_miso_byte = Some(shift_out); bytes_exchanged++; bit_count = 0; shift_in = 0. Falling edge / SCLK while deselected = no-op except update sclk_last.
   - On SELN going high (deselect edge): reset bit_count and shift_in (matches real chips that abort mid-byte on CS rise).
   - Re-export from src/cpu/mod.rs.

2. Add field `pub spi: SpiBusState` to IoState (mirror i2c field). Skip serde on it (matches i2c bus state).

3. Wire src/cpu/state.rs write_io: after persisting the master line value, call self.io.spi.step(self.io.master_sclk, (self.io.master_mosi >> bit_index) & 1, ...) — but actually we need to pass a single bit-of-mosi which is determined by the bit_count. Simpler: step takes the full master_mosi byte and the state machine extracts the next-to-shift bit by tracking bit_count internally. Either is fine; pick whichever reads cleaner.

   The plan §4.6 says: "Mode 0 (CPOL=0, CPHA=0): MOSI is set on falling edge, sampled on rising." Our spixchg.s drives:
       SDA = bit (master sets MOSI)
       SCL = 1   (rising edge — slave samples MOSI)
       SCL = 0
   So on SCL rising edge, sample master_mosi. The bit being sampled is the MSB of whatever the master most recently wrote — but spixchg writes a single bit at a time to MOSI (not a byte). Looking at spixchg.s more carefully: the master writes the MSB of its data byte to IO_SPI_DATA on each iteration (`sb r1, 0(r2)`), then pulses SCLK. So the bit at master_mosi & 1 is the bit-being-sent.

   Simpler model: step takes the current master_mosi (u8) but uses only its bit 0 (since the guest only meaningfully writes one bit at a time as 0 or 1 to IO_SPI_DATA in spixchg). Compatible with the I2C convention where IO_I2C_SCL/SDA are bit-0 lines.

4. Unit tests in src/cpu/spi_bus.rs:
   - exchange_one_byte: write SELN=0, then 8 cycles of (set DATA=bit, SCL=1, SCL=0), assert last_mosi_byte = expected.
   - sclk_with_seln_high_does_nothing: bit_count stays 0.
   - mid_byte_deselect_resets: 4 bits in, then SELN=1, then SELN=0 again — bit_count back to 0.
   - bytes_exchanged increments per byte.

5. Drop the now-redundant tmp125_drives_some_clocks assertion if it gets noisy; alternatively extend it to assert spi.bytes_exchanged > 0.

Out of scope:
- SpiDevice trait (step C.5).
- Any actual MISO byte (shift_out stays 0 for now; on master read the wire returns 0).

Done when:
- src/cpu/spi_bus.rs landed with state + 4+ unit tests.
- IoState carries the bus state; write_io drives it on every IO_SPI_SCLK or IO_SPI_DATA write.
- tmp125_drives_some_clocks (or its replacement) asserts at least one byte exchanged.
- cargo test --workspace passes; clippy clean.

Next step: --next-slug spi-device-trait-and-echo — Phase C.4, the SpiDevice trait + a universal "echo" test slave + SpiHandle, mirroring I2C step B.1.