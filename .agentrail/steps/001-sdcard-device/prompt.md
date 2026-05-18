Implement Step 1 of brief 8 — SPI SD Card device (SD-mode protocol).

**Read first:**
- `docs/briefs/8-dcemu-spi-sdcard-and-nor-flash.md` § "Step 1: SDCardDevice (SPI mode)" — full spec.
- `src/peripherals/spi/devices/tmp125.rs` — existing SPI device shape to mirror.
- `src/peripherals/spi/devices/echo.rs` — simplest example.
- `src/peripherals/i2c/devices/ds1307.rs` — closest analogue for "user-controlled initial state" via `?file=` param.

**What:**
- Add `src/peripherals/spi/devices/sdcard.rs` with `SdCardDevice` implementing the SPI-mode command subset: CMD0, CMD8, CMD55+ACMD41, CMD16, CMD17, CMD24.
- Image: optional `?file=<path>` registry param; absent = 1 MiB in-memory zeros. Writes propagate back to the host file when present.
- 512-byte block read/write. CMD17 emits ≥8 busy `0xFF` bytes, then `0xFE` data token, 512 data, two `0xFF` dummy CRC bytes. CMD24 awaits `0xFE` from master, buffers 512, writes back, emits `0xE5` accepted.
- Re-export from `src/peripherals/spi/devices/mod.rs` and `src/peripherals/spi/mod.rs`.
- `SdCardHandleExt`: `image()`, `size()`, `last_accessed_sector()`, `replace_image(Vec<u8>)`.
- Registry parser: `sdcard@cs=<n>[?file=<path>]` — match the existing SPI device pattern. Pick the next free CS (TMP125 occupies CS=1; sdcard defaults to CS=2). Update CLI `--spi-device` help text.
- Skip-on-this-pass (document): pre-CMD0 dummy clocks, CRC validation on commands, CRC computation on responses (emit 0xFF), multi-block CMD18/CMD25.

**Tests** (per the brief, §Step 1):
- `cmd0_returns_idle`
- `cmd8_echo`
- `acmd41_transitions_to_ready`
- `cmd17_reads_known_sector`
- `cmd24_writes_persists`
- `cmd24_writes_to_file` (tempfile-backed)
- Plus an integration test in `tests/spi.rs` exercising `--spi-device sdcard@cs=2?file=<tmpfile>` end-to-end.

**Acceptance:**
- `cargo test --workspace` green.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Exactly ONE commit on this step: `feat(spi): SdCardDevice — SD-mode card reader`.
- Stop after `agentrail complete` — Step 2 (W25Q32) is the next saga step, not this one.

Out of scope per the brief: SDHC vs SDSC beyond CMD8, multi-block, dual/quad SPI modes, CRC validation.