Phase C.5 — TMP125 SPI device (mirror of I2C B.2: TMP101).

Plan ref: docs/feature-i2c-spi-emu.md sec 5.4.

What to do:
1. New src/peripherals/spi/devices/tmp125.rs:
   - Tmp125Device with set_temperature(f32), temperature() -> f32, plus a 16-bit register that the chip exposes on read (10 bits of temperature in upper bits, lower bits are status/sign per datasheet).
   - on_select pre-loads byte 0 of the temperature register; on_byte returns byte 1 then byte 0 again (continuous read on the real chip).
   - Tmp125HandleExt with set_temperature/temperature/set_resolution sugar over handle.with(|d| ...).
2. Wire into devices/mod.rs and registry.rs so build_spi_device("tmp125") and "tmp125?temp=23.5" both work.
3. tests/spi.rs E2E: load examples/spi/tmp125/tmp125.lgo, attach Tmp125Device with set_temperature(25.0), run for ~200k instructions, assert UART output starts with the expected fixed-width temperature string ("DD.DD\n" shape — exact value depends on demo's printf precision; mirror the i2c tmp101 e2e).
4. Add device unit tests for Tmp125Device covering register layout, sign extension, set_temperature round-trip via on_select / on_byte sequence.

Done when:
- tmp125 spec parses through build_spi_device.
- E2E test against tmp125.lgo passes with deterministic temperature output.
- cargo test --workspace passes; clippy clean.