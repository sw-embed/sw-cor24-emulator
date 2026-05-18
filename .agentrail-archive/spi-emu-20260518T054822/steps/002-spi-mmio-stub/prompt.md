Phase C.1 — SPI MMIO stub at 0xFF0030 / 0xFF0031 / 0xFF0032.

Mirror of I2C step 002. Land just the constants and trivial round-trip storage; no shift register or device dispatch yet.

Plan ref: docs/feature-i2c-spi-emu.md sec 4.6 (SPI bus model), sec 4.7 (IoState additions).

What to do:
1. src/cpu/state.rs:
   - Add three pub const u32:
       IO_SPI_DATA = 0xFF0030  // MOSI write, MISO read
       IO_SPI_SCLK = 0xFF0031  // SCLK line
       IO_SPI_SELN = 0xFF0032  // bit 0 = device 0 select (active low)
   - read_io match arms returning 0 for now (no slave; TODO note that
     IO_SPI_DATA reads will return slave_miso once the bus state
     machine lands).
   - write_io match arms storing nothing yet (or persisting low bit
     into IoState fields if you want to land that here — see the
     prompt in step 003 master-line-state which would otherwise do it).
   - Two unit tests: read returns 0; writes do not crash.

2. tests/spi.rs gets one new test: tmp125_runs_with_stub_mmio still
   passes (already in step 001), confirming the spixchg loop now polls
   the new MMIO addresses without crashing. No behaviour change.

Out of scope:
- Master-line state (step 003).
- Shift-register state machine (step 004).
- SpiDevice trait (step 005).

Done when:
- IO_SPI_* constants defined and exported as appropriate.
- Trivial read returns 0; write is no-op (or stores low bit, your call —
  if you do, document that step 003 already exists).
- cargo test --workspace passes.
- clippy clean.

Next step: --next-slug spi-master-line-state — persist master_mosi /
master_sclk / master_seln in IoState; reads return what was written.