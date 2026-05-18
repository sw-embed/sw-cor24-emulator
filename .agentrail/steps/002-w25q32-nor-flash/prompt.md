Implement Step 2 of brief 8 — Winbond W25Q32 NOR flash (4 MiB).

**Read first:**
- `docs/briefs/8-dcemu-spi-sdcard-and-nor-flash.md` § "Step 2: W25q32Device" — full spec including the command table.
- The just-shipped `src/peripherals/spi/devices/sdcard.rs` from saga step 1 — mirror its `?file=` persistence pattern, registry shape, and HandleExt style.
- `src/peripherals/spi/devices/tmp125.rs` — existing SPI shape.

**What:**
- Add `src/peripherals/spi/devices/w25q32.rs` with `W25q32Device`. Exact 4 MiB image (0x40_0000 bytes). Default state: all `0xFF` (erased flash baseline).
- Implement the W25Q32 command subset per the brief's table:
  - `0x9F` JEDEC ID → `0xEF 0x40 0x16` (Winbond W25Q32 signature)
  - `0x05` Read Status Register 1 (bit 0 = WIP, bit 1 = WEL)
  - `0x06` Write Enable (set WEL)
  - `0x04` Write Disable (clear WEL)
  - `0x03` Read Data — 3-byte addr, auto-increment until CS deselect
  - `0x02` Page Program — 3-byte addr + ≤256 data; requires WEL=1; sets WIP for ~1024 byte-clocks
  - `0x20` Sector Erase (4 KiB aligned, ~4096 byte-clocks WIP)
  - `0xD8` Block Erase (64 KiB aligned, ~16384 byte-clocks WIP; document chosen duration)
  - `0xC7`/`0x60` Chip Erase (optional WIP delay — document choice; OK to skip)
- WIP/WEL: don't `thread::sleep`; track `wip_clocks_remaining: u32` decrementing on every byte clocked through the bus, then clear WIP when it hits zero. WEL clears automatically after each successful program/erase.
- Honor the "1→0 transitions only" rule on Page Program — programming over unerased data should silently AND (faithful behavior); document the choice in the doc-comment.
- `?file=<path>` registry param: load on attach, sync to disk on each successful program/erase. Absent = in-memory.
- Re-export from `mod.rs` files. CLI help text update for `w25q32@cs=<n>[?file=<path>]`.
- `W25q32HandleExt`: `image()`, `jedec_id()`, `wip()`, `wel()`, `last_accessed_address()`, `replace_image(Vec<u8>)`, `erase_chip()`.

**Tests** (per the brief, §Step 2):
- `jedec_id_signature`
- `read_uninitialized_returns_ff`
- `page_program_requires_wel`
- `page_program_after_erase`
- `page_program_one_to_zero_rule`
- `sector_erase_clears_4kb`
- `block_erase_clears_64kb`
- `wip_clears_after_clocks`
- `image_persists_to_file`
- Plus an integration test in `tests/spi.rs` with the CLI flag.

**Acceptance:**
- `cargo test --workspace` green.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Exactly ONE commit on this step: `feat(spi): W25q32Device — Winbond W25Q32 NOR flash (4 MiB)`.
- This is the saga's final step — pass `--done` to `agentrail complete`.
- After `complete`, rename branch to `pr/spi-sdcard-and-nor-flash` via `dg-mark-pr`.

Out of scope per the brief: dual/quad SPI, 4-byte addressing, CRC validation.