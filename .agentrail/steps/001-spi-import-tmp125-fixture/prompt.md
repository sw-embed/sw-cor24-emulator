Phase C.0 — Import tmp125 SPI demo + fix tmp101 Makefile drive-by.

Mirror of I2C step 001. Source at work/research/i2cspi/i2cspi/tmp125/.
Plan ref: docs/feature-i2c-spi-emu.md sec 3.1, 3.3, 4.6, 5.2.

What to do:
1. Drive-by fix examples/i2c/tmp101/Makefile (uses removed --assemble flag): replace
   COR24RUN := cor24-run; ... $(COR24RUN) --assemble $(S) $(BIN) $(LST)
   with cor24-asm $(S) --bin $(BIN) --listing $(LST). Update header comment to reflect.

2. Copy work/research/i2cspi/i2cspi/tmp125/{libspi.c,libspi.h,spiio.h,spixchg.s,tmp125.c,tmp125.pdf} to examples/spi/tmp125/. Adjust:
   - libspi.c: K&R-style declaration void spiseln(d) char d; -> ANSI prototype.
   - tmp125.c: drop printf (no stdio in tc24r). Replace with direct UART output mirroring tmp101.c printtemp pattern (10-bit signed value in 0.25C steps -> "DD.DD\n").
   - tmp125.c: register int t -> int t; while (t--); -> while (t--) {} (tc24r requirements per i2c step 001 lessons).
   - tmp125.c: amalgamate via #include "libspi.c" so tc24r single-translation-unit can link.
   - spixchg.s: keep .text/.globl as-is (cor24-asm tolerates them).
   - The C file must produce a single .s; spixchg.s is concatenated/included separately. Investigate cor24-asm multi-input support; if absent, prepend spixchg.s into the tc24r-emitted .s before assembling, OR shell-cat them together in the Makefile.

3. New Makefile examples/spi/tmp125/Makefile, mirroring the (now-fixed) i2c tmp101 one:
   tmp125.s    <- tc24r tmp125.c -o tmp125.s
   tmp125_full.s <- cat spixchg.s tmp125.s
   tmp125.lgo  <- cor24-asm tmp125_full.s -o tmp125.lgo

4. Build the .lgo locally and commit it as the test fixture.

5. examples/spi/tmp125/.gitignore: ignore intermediate .s if appropriate.

6. New tests/spi.rs scaffolding mirror of tests/i2c.rs:
   - test_tmp125_fixture_loads — load .lgo via EmulatorCore::load_lgo, assert non-empty.
   - test_tmp125_runs_with_stub_mmio — resume + run for 100K instructions, assert no halt/crash. The CPU will spin in spixchg waiting for SCLK reads since no bus emulation exists; test just confirms no halt/invalid-instruction.

7. Update scripts/rebuild-i2c-fixtures.sh to also rebuild SPI fixtures (rename to rebuild-fixtures.sh OR add a sibling script — pick whichever matches the existing pattern). tc24r-on-PATH skip stays.

Out of scope:
- IO_SPI_* MMIO state in IoState (step 2).
- SpiDevice trait or any device (step 5).
- Bus state machine (step 4).
- Anything that requires src code in src/ to change (apart from existing module exports).

Tests:
- cargo test --workspace passes (the new tests are load-only + crash-free smoke).

Done when:
- examples/spi/tmp125/ committed (sources + .lgo).
- examples/i2c/tmp101/Makefile no longer references the removed --assemble flag.
- tests/spi.rs scaffolded and green.
- Rebuild script handles SPI fixtures.
- cargo test --workspace green.

Next step: --next-slug spi-mmio-stub — Phase C.1, stub MMIO at 0xFF0030/31/32 (data, sclk, seln). Master-side reads return last-written; no bus state machine yet.