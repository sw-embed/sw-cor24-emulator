Phase C.2 — SPI master line state.

Mirror of I2C step 003 master-line-state. Persist what the master writes to IO_SPI_DATA / IO_SPI_SCLK / IO_SPI_SELN so reads return what was written (until step C.3 wires the shift register and the slave-driven MISO).

Plan ref: docs/feature-i2c-spi-emu.md sec 4.6 (SPI bus model), 4.7 (IoState additions).

What to do:
1. src/cpu/state.rs IoState struct: add three fields:
       pub master_mosi: u8,    // last byte written to IO_SPI_DATA
       pub master_sclk: bool,  // last bit written to IO_SPI_SCLK
       pub master_seln: bool,  // last bit written to IO_SPI_SELN (true = deselected)
   Initialise master_seln = true (idle: nothing selected); master_sclk = false; master_mosi = 0.

2. read_io match arms:
       IO_SPI_DATA => self.io.master_mosi,
       IO_SPI_SCLK => self.io.master_sclk as u8,
       IO_SPI_SELN => self.io.master_seln as u8,

3. write_io match arms:
       IO_SPI_DATA => self.io.master_mosi = value,
       IO_SPI_SCLK => self.io.master_sclk = (value & 1) != 0,
       IO_SPI_SELN => self.io.master_seln = (value & 1) != 0,

4. Update the existing test_spi_mmio_reads_zero_at_stub to test_spi_idle_state_after_reset (rename: SCLK and DATA still 0 at reset; SELN now reads 1 = deselected).

5. test_spi_mmio_writes_are_noops_at_stub becomes test_spi_master_line_roundtrip — write a byte to DATA reads back; write 1/0 to SCLK and SELN round-trip on bit 0.

6. tests/spi.rs: extend tmp125_runs_with_stub_mmio (or add a new tmp125_drives_some_clocks test) that runs the fixture for ~50K instructions and asserts the SCLK line was driven low+high at least once during the run. Confirms the spixchg loop is making bus progress now that writes persist.

Out of scope:
- Shift-register state (step C.3).
- SpiDevice trait or any slave (step C.5).

Done when:
- IoState has the three new fields.
- Reads return what was written; writes persist low bit (SCLK/SELN) or full byte (DATA).
- Existing SPI MMIO tests upgraded to round-trip versions.
- New tmp125-driven test asserts at least one SCLK transition.
- cargo test --workspace passes.

Next step: --next-slug spi-bus-shift-register — Phase C.3, the SPI shift-register state machine. On SCLK rise with SELN=0, sample MOSI, advance bit count; on every 8 bits trigger a (later: SpiDevice) handler. No device yet — just the state.