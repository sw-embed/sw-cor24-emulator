Phase C.4 — SpiDevice trait + echo test slave + SpiHandle.

Mirror of I2C step 005 (B.1: device trait + add1 + handle). For SPI the device trait is simpler (no addressing, no ACK/NAK semantics): on_select / on_byte / on_deselect / on_tick.

Plan ref: docs/feature-i2c-spi-emu.md sec 5.2.

What to do:
1. New module src/peripherals/spi/ mirroring src/peripherals/i2c/:
   - mod.rs (re-exports)
   - device.rs: SpiDevice trait. on_select(); on_byte(mosi: u8) -> u8 (returns the MISO byte for the SAME 8-clock exchange — but really the slave's pre-loaded buffer; see timing note in step 3); on_deselect(); on_tick(); name() default. Send + 'static.
   - handle.rs: SpiHandle<D> with with(|d| ...). No address mutation — SPI has no per-device address; one slave per bus today, multi-slave bitmask is plan sec 9 future work.
   - registry.rs: build_spi_device(spec) -> Arc<Mutex<dyn SpiDevice>> by spec like "echo" or "echo?seed=0xAA" — single-slave so no @addr.
   - devices/mod.rs + devices/echo.rs: EchoDevice — universal SPI test slave that returns the previous MOSI byte each time (one-byte echo delay, matching how a hardware shift register actually works). Handle exposes peek/poke for the buffered byte.

2. Bus integration in src/cpu/spi_bus.rs:
   - SpiBusState gets a single Option-typed slave slot rather than a full AddressMap. SPI is single-slave today; multi-slave is a future SELN bitmask (plan sec 9).
   - Add a field like: `pub device: Option<Arc<Mutex<dyn SpiDevice>>>` skipped from serde.
   - On byte completion, if a device is attached, call dev.on_byte(mosi_byte). The returned byte is the NEXT byte to drive on MISO — not this one (timing: the slave needed to know the byte to send before the master clocked bit 0 of the current byte).
   - Pre-load on SELN falling: on_select(); pre-load shift_out from a per-device "next byte" register (the device's idea of "what to send first"). EchoDevice's on_select returns its buffer; on_byte(mosi) latches mosi and returns the previous buffer.
   - On SELN rising: on_deselect().

3. EmulatorCore::attach_spi_device::<D>(dev) -> SpiHandle<D> mirror of attach_i2c_device. attach_spi_device_shared(arc) for the registry path. detach_spi_devices clears the slot.

4. Tests:
   - EchoDevice unit tests: name, on_byte buffering, poke/peek round-trip, on_select pre-load.
   - Bus + EchoDevice integration in spi_bus.rs#cfg(test): drive 3 bytes via direct step calls; assert MISO byte sequence is the previous MOSI byte (with first byte being the seed).
   - SpiHandle.with round-trip in handle.rs#cfg(test).
   - tests/spi.rs gets a synthetic-bus harness mirroring the i2c add1_full_write_then_read_cycle pattern: attach EchoDevice, hand-drive SCLK/SELN/DATA to clock 16 bits, assert read-back bytes match the echo pattern.

5. Drive-by: with SpiDevice attached, the existing tmp125_exchanges_bytes test now sees last_miso_byte = Some(slave_byte) instead of Some(0). Update its assertion if the test is too strict — switch from "last_miso_byte == Some(0)" to "last_miso_byte.is_some()".

Done when:
- SpiDevice trait + EchoDevice + SpiHandle<D> + EmulatorCore::attach_spi_device.
- Bus dispatches on byte boundaries; MISO returns slave-driven bytes.
- cargo test --workspace passes; clippy clean.

Next step: --next-slug spi-tmp125-device — Phase C.5 mirror of i2c B.2: TMP125 device modelling the 16-bit temperature register, set_temperature(f32), end-to-end test against tmp125.lgo asserting "DD.DD\n" on the UART.