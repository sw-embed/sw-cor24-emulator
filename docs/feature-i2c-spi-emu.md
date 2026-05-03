# COR24 emulator — I2C and SPI peripheral support

Plan for adding I2C support to the COR24 ISA emulator now, with a parallel
design sketched for SPI to add later. Targets the existing Rust emulator
at `sw-cor24-emulator.git` (the same crate that already emulates LED/switch
and UART at the addresses mapped by the COR24-TB FPGA design).

## 1. Three-layer scope

This work has three distinct deliverables, and most design choices fall
out of getting the boundary between them right:

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: Guest applications                                     │
│   C programs + libi2c / libspi running on the COR24             │
│   (already exist in i2cspi/tmp101 and i2cspi/tmp125)            │
└──────────────────────┬──────────────────────────────────────────┘
                       │ MMIO writes/reads to GPIO-style addresses
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: Bus MMIO emulation (in CPU emulator core)              │
│   Models the FPGA's I2C/SPI line registers exactly. Reconstructs│
│   logical bus events from line transitions. Routes events to    │
│   whatever device(s) are currently attached.                    │
└──────────────────────┬──────────────────────────────────────────┘
                       │ Bus event API: on_start / on_byte / on_stop / ...
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: Pluggable virtual devices                              │
│   Implementations of an I2cSlave / SpiSlave trait for each chip │
│   we want to model: TMP101, TMP125, DS3231 RTC, BME280, an LCD, │
│   rotary encoder, EEPROMs, etc. New devices = new files.        │
└─────────────────────────────────────────────────────────────────┘
```

Layers 1 and 3 grow independently over time (more demos, more
chip models). Layer 2 is small, central, and ideally written once.

The unstated fourth concern — **demoability** — is that layer 1 examples
are how a user learns to write COR24 I2C/SPI code, and layer 3 device
models are how that code can run anywhere (CI, browser, laptop) without
real hardware. So both layers double as documentation when the suite is
representative.

## 2. Goals

- Run the existing `i2cspi/tmp101/tmp101.lgo` demo end-to-end in the
  emulator and produce the same temperature output as the FPGA board.
- Match the FPGA MMIO layout exactly (`0xFF0020`/`0xFF0021` for I2C,
  `0xFF0030..0xFF0032` for SPI) so guest binaries are bit-identical
  between FPGA and emulator.
- Make the device layer a **public, documented extension point**: a
  third party (or future-you) writes one Rust file implementing a trait
  and registers it; no edits to the bus core, no fork.
- Expose a chronological transaction log (analogous to `UartLog`) so the
  CLI and Web UI can show what's happening on the bus.
- Keep SPI design symmetric with I2C so phase 2 is mostly mechanical.

Non-goals: cycle-accurate timing, multi-master arbitration, 10-bit I2C
addresses, SPI modes other than CPOL=0/CPHA=0 (which is what
`spixchg.s` implements).

## 3. Layer 1 — Guest applications

### 3.1 What exists today

```
i2cspi/tmp101/
  i2cio.h         #define I2CBASE 0xFF0020 / SCL=+0 / SDA=+1
  libi2c.c        i2cstart, i2cstop, i2cwrite, i2cread (bit-banged)
  libi2c.h        public interface
  tmp101.c        configures TMP101 at 0x4A, prints temperature in °C
  tmp101.lgo      assembled binary — the emulator's first integration test

i2cspi/tmp125/
  spiio.h         #define SPIBASE 0xFF0030 / MISO=+0 / MOSI=+0 / SCLK=+1 / SELN=+2
  libspi.c        spiseln helper
  spixchg.s       8-bit unrolled SPI exchange in assembly
  tmp125.c        prints temperature from a TMP125 SPI sensor
  tmp125.lgo      assembled binary — phase-2 integration test
```

These are **canonical examples** for the platform: any new I2C/SPI demo
follows the same structure (`<chip>io.h` defines the MMIO base,
`lib<bus>.c` provides the protocol, `<demo>.c` does the chip-specific
work). They double as the "how to write a new bus app" tutorial.

### 3.2 What we'll add

- A short author's-guide markdown (probably in `docs/` of the emulator
  repo, or alongside the demos) that walks through writing a new I2C
  demo using `tmp101.c` as the template, and points at the matching
  device model so the user can run it in the emulator.
- A second I2C demo against a different device class (an EEPROM read or
  an RTC time read) so the suite shows that more than one device shape
  is supported. This forces the device API to be general.

### 3.3 What we will *not* do at the application layer

No changes to `libi2c.c` / `libspi.c` / `spixchg.s` — the whole point
of MMIO-accurate emulation is that the same source runs on FPGA and
emulator. If a demo needs different code on emulator vs. FPGA, the
abstraction has leaked and we should fix the emulator instead.

## 4. Layer 2 — Bus MMIO emulation

### 4.1 How guest code drives the bus

Both interfaces are **bit-banged GPIO**, not register-driven peripherals.
The CPU pokes individual line states and clocks the bus itself. This is
materially different from the existing UART, where the CPU writes one
byte to `IO_UARTDATA` and the emulator reacts to that byte.

I2C — the master writes 1 to release a line and 0 to drive it low; reads
return the actual line state (open-drain wired-AND). `clkhiw()` writes
SCL=1 then *polls* SCL until it reads back 1 — the standard
clock-stretching check, so a slave must be able to hold SCL low.

For I2C reads, the master writes SDA=1 (releases the line) and samples
SDA during the SCL-high window — meaning the slave drives SDA by causing
those reads to return 0 for the bits it wants to send.

SPI is simpler: write MOSI, toggle SCLK, sample MISO. Mode 0, MSB-first,
single SS line.

### 4.2 The emulator can't just "respond when the CPU writes a byte"

It sees individual line transitions and must **reconstruct logical bus
events** (START, address+RW, byte-in, byte-out, ACK, STOP for I2C;
bit-shift on SCLK edge for SPI) from the physical transitions, then
route those events to a virtual device.

### 4.3 Where it slots in

The Rust emulator's I/O dispatch already has the right shape — a flat
`match addr { ... }` in `CpuState::read_io` / `write_io`
(`src/cpu/state.rs`). Extending it follows the existing UART pattern:

1. Add fields to `IoState` for the new peripheral's persistent state.
2. Add MMIO address constants (`IO_I2C_SCL`, `IO_I2C_SDA`, etc.).
3. Add match arms in `read_io` / `write_io`.
4. If the peripheral needs per-instruction "time" advancement (like the
   UART's TX-busy countdown), add a `*_tick()` method called alongside
   `uart_tick()` in the executor loop.
5. Surface configuration and observability through `EmulatorCore`
   (`src/emulator.rs`) so `cli/src/run.rs` and the Web UI's `app.rs`
   can use it.

Nothing in the CPU core, decode ROM, or instruction executor needs to
change — this is purely an I/O peripheral addition.

### 4.4 I2C bus model

Track four bits, two per line:

| field             | meaning                                    |
|-------------------|--------------------------------------------|
| `master_scl`      | What the CPU last wrote to `IO_I2C_SCL`    |
| `master_sda`      | What the CPU last wrote to `IO_I2C_SDA`    |
| `slave_scl_pull`  | True if any attached device is stretching  |
| `slave_sda_pull`  | True if any attached device is driving low |

Effective line states (open-drain wired-AND):

```
scl_line = master_scl  & !slave_scl_pull
sda_line = master_sda  & !slave_sda_pull
```

Reads from `IO_I2C_SCL` / `IO_I2C_SDA` return the effective line state in
bit 0. Writes update the master driver state and then advance the state
machine on every transition. This handles `clkhiw()` correctly: with no
device stretching, the master sees SCL=1 immediately after writing 1.

### 4.5 I2C state machine

Driven by `(scl_line, sda_line)` transitions, not by writes:

```
Idle               // both lines high, no transaction
Started            // saw START; waiting for address bits
RxByte(bits, n)    // shifting in a byte from master (addr or write data)
AckMasterToSlave   // device decides to ACK/NAK after addressed/written
TxByte(bits, n)    // shifting out a byte to master (read data)
AckSlaveToMaster   // master ACKs (or NAKs to end read)
Stopped            // saw STOP — back to idle next cycle
```

Edge detection rules (standard I2C):

- **START**: SDA falls while SCL is high → `Idle | * → Started`
- **STOP**:  SDA rises while SCL is high → `* → Stopped → Idle`
- **Bit clocked in**: SCL rises → if in `RxByte(_,n)`, shift in `sda_line`
- **Bit clocked out**: master drives SCL low after sampling → advance bit
- **Repeated START**: handled by the START rule from any non-Idle state

After 8 bits in `RxByte`, transition to `AckMasterToSlave`. The first
byte of every transaction is `(addr<<1)|rw`; when that completes, look
up the device with the matching 7-bit address and route subsequent
events to it.

### 4.6 SPI bus model (phase 2)

Skeletally the same but simpler — no addressing, no START/STOP, no
open-drain. Just a shift register clocked by SCLK while SELN is low.

```
master_mosi, master_sclk, master_seln_n   // CPU writes
slave_miso                                 // selected device drives this
```

`IO_SPI_DATA` (0xFF0030) reads `slave_miso`, writes `master_mosi`.
`IO_SPI_SCLK` (0xFF0031) — only writes matter; read returns last value.
`IO_SPI_SELN` (0xFF0032) — bit 0 selects device 0 (active low).

State machine: on SCLK rising edge with SELN=0, shift `master_mosi`
into the master input register; ask the selected device for its next
MISO bit. Mode 0 (CPOL=0, CPHA=0): MOSI is set on falling edge, sampled
on rising.

### 4.7 IoState additions

```rust
pub struct I2cBusState {
    pub master_scl: bool,
    pub master_sda: bool,
    pub phase: I2cPhase,
    pub shift: u8,
    pub bit_count: u8,
    pub current_target: Option<u8>,
    pub current_dir: I2cDir,
    pub log: I2cLog,
    // Devices live here, but see "Open question: serialization" below.
    pub devices: Vec<Box<dyn I2cDevice>>,
}
```

`I2cBusState` is **not** `Serialize` — trait objects don't fit the
existing serde derive on `IoState`. Mark the bus state `#[serde(skip)]`
and re-attach devices after load. (Web UI's `WasmCpu` clones state but
doesn't currently persist across runs, so this is fine.)

### 4.8 Per-instruction tick

I2C strictly doesn't need a tick (everything advances on writes), but
some devices simulate work that takes time (TMP101 conversion time,
EEPROM write time). Add a tick hook on the device trait and call it
from the executor loop alongside `uart_tick()`.

For the first cut, ticks are optional: a device that doesn't override
`on_tick` is just instant-response.

## 5. Layer 3 — Pluggable device extension API

This is the part the user will interact with most. A new device should
be one Rust file plus one registry line, no other changes. The trait is
the public surface.

### 5.1 The I2C device trait

```rust
/// Abstracts a virtual I2C slave device attached to the emulator's bus.
/// Implementations live in src/peripherals/i2c/devices/.
pub trait I2cDevice: Send {
    /// 7-bit I2C address this device responds to.
    fn address(&self) -> u8;

    /// Optional human-readable name (used in logs and CLI).
    fn name(&self) -> &str { "i2c-device" }

    /// Bus state events. Default impls let simple devices override only
    /// what they need.
    fn on_start(&mut self) {}
    fn on_write_byte(&mut self, byte: u8) -> Ack { Ack::Nak }
    fn on_read_byte(&mut self) -> u8 { 0xFF }
    fn on_master_ack(&mut self) {}
    fn on_master_nak(&mut self) {}
    fn on_stop(&mut self) {}

    /// Called once per CPU instruction. Use sparingly — only for devices
    /// that model time (conversion delay, watchdog, etc.).
    fn on_tick(&mut self) {}

    /// Optional clock stretching. Return true to hold SCL low.
    fn stretching_scl(&self) -> bool { false }
}

pub enum Ack { Ack, Nak }
```

### 5.2 The SPI device trait (phase 2)

```rust
pub trait SpiDevice: Send {
    fn name(&self) -> &str { "spi-device" }
    fn on_select(&mut self) {}
    /// Simultaneous shift: receive a byte from MOSI, return the byte
    /// that was on MISO during the same 8 clocks.
    fn on_byte(&mut self, mosi: u8) -> u8;
    fn on_deselect(&mut self) {}
    fn on_tick(&mut self) {}
}
```

### 5.3 Device registry & construction

A small string-keyed registry that maps device names to constructors
parameterized by config:

```rust
pub fn build_i2c_device(spec: &str) -> Result<Box<dyn I2cDevice>, String>
// e.g. "tmp101@0x4A"
//      "tmp101@0x4A?temp=23.5"
//      "ds3231@0x68?epoch=2026-05-03T12:00:00Z"
//      "logger@0x50"
```

Adding a chip:

1. New file `src/peripherals/i2c/devices/<chip>.rs` with a struct
   implementing `I2cDevice`.
2. One line in `src/peripherals/i2c/registry.rs` mapping `"<chip>"` to
   a constructor closure.
3. Tests for the device in the same file.

That's the entire integration cost. Bus core, CPU, executor, CLI, UI
all unchanged.

### 5.4 First device set

Phase 1, in priority order:

| Device       | Why                                                   |
|--------------|-------------------------------------------------------|
| `tmp101`     | Validates the existing demo binary end-to-end         |
| `logger`     | Bit-bucket device; records every event for tests/UI   |
| `eeprom`     | Read/write semantics — exercises a different shape    |
| `ds3231`     | RTC — multi-byte register file, common in projects    |

Phase 2 / SPI:

| Device       | Why                                                   |
|--------------|-------------------------------------------------------|
| `tmp125`     | Validates the existing SPI demo                       |
| `mcp23s17`   | Generic SPI GPIO expander — useful in many demos      |

Future / nice-to-have: BME280 (I2C+SPI), HD44780/SSD1306 LCD,
rotary encoder via I2C expander. None of these block phase 1.

### 5.5 Documentation deliverable

A `docs/extending-i2c.md` file in the emulator repo that:

- Walks through writing a new device from scratch using a minimal toy
  chip (e.g. an "echo" device).
- Documents every method on the trait with timing/sequencing notes.
- Links to the actual chip implementations as worked examples.

Without this doc, the extension API isn't really a public API.

## 6. EmulatorCore and CLI surface

Mirror the UART API in `src/emulator.rs`:

```rust
impl EmulatorCore {
    pub fn attach_i2c_device(&mut self, dev: Box<dyn I2cDevice>);
    pub fn detach_i2c_devices(&mut self);
    pub fn i2c_log(&self) -> &I2cLog;
    pub fn format_i2c_log(&self) -> String;

    // Phase 2:
    pub fn attach_spi_device(&mut self, dev: Box<dyn SpiDevice>);
    pub fn spi_log(&self) -> &SpiLog;
}
```

CLI flags in `cli/src/run.rs`:

```
--i2c-device tmp101@0x4A             # attach a TMP101 at addr 0x4A
--i2c-device 'tmp101@0x4A?temp=25.0' # with config
--i2c-device logger@*                # passive logger on every address
--dump-i2c                           # print transaction log on exit

# Phase 2:
--spi-device tmp125
--dump-spi
```

`--i2c-device` can be repeated. The flag value goes through
`build_i2c_device(spec)` so the parser logic lives in one place.

Log entries are at the **transaction level**, not the bit level — a
START is one entry, a complete addressed byte-write is one entry, etc.
Bit-level traces are useful for debugging the bus core itself but not
day-to-day; gate them behind `--dump-i2c-bits` or similar.

## 7. Tests

Three layers, mirroring the architecture:

1. **Bus state machine unit tests** — feed sequences of `(scl, sda)`
   writes, assert phase transitions and decoded byte values. No CPU.
   Pure logic, fast.

2. **Device unit tests** — call trait methods directly, assert
   responses. (TMP101: pointer-register behavior. EEPROM: address
   wrap. RTC: multi-byte register reads.)

3. **End-to-end integration tests** — load `tmp101.lgo`, attach a
   `Tmp101` device at `0x4A`, run for N instructions, assert the UART
   output contains the expected `"%.2f\n"` line for the configured
   temperature. This is the single most important test — if it passes,
   the whole stack works (libi2c bit-banging, bus state machine,
   device model, UART output, printf).

The Makefile in `i2cspi/tmp101` already produces the `.lgo`; the
emulator already has the loader. Integration test is just wiring.

## 8. Implementation order (I2C first)

Each step is a single small commit; the suite stays green at every step.

1. **Constants and stub I/O** — add `IO_I2C_SCL`, `IO_I2C_SDA` to
   `state.rs`; reads return 1 (idle bus), writes are no-ops. Verify
   `tmp101.lgo` doesn't crash; it'll spin in `clkhiw()` since nothing
   ACKs.

2. **Master line state** — store `master_scl` / `master_sda` in
   `IoState`, return them on read. `clkhiw()` returns immediately. The
   driver runs to STOP, but no device responds, so reads are 0xFF and
   ACK is NAK.

3. **Bus state machine** — implement edge detection and the phase
   enum. No devices yet; just verify the state machine recognizes
   START, address byte, STOP from the bit stream. Unit tests at this
   layer.

4. **Device trait + LoggingDevice** — define `I2cDevice`, wire device
   lookup on address-byte complete, route events through the device.
   Verify the logger sees the right address+RW and bytes for tmp101.

5. **TMP101 device** — model enough of the chip (config register,
   temperature register, pointer register) for the demo. End-to-end
   test asserts the printed temperature.

6. **EmulatorCore + CLI plumbing** — `attach_i2c_device`,
   `--i2c-device`, `--dump-i2c`. UI changes can come later; CLI is
   enough to validate.

7. **Registry + second device** — implement `build_i2c_device` and add
   one more device (EEPROM or DS3231) to prove the trait is general
   enough. This is the gate before declaring the extension API "public".

8. **Extension docs** — `docs/extending-i2c.md` with a worked example.
   Without this, the API isn't really an API.

9. **Web UI surface** (optional, separable) — a panel showing the I2C
   log alongside the UART log, plus an attach/detach control.

After this lands, phase 2 (SPI) follows the same shape, replacing the
state machine with the simpler shift-register model and re-using the
device-attachment plumbing nearly verbatim.

## 9. Open questions

- **Multi-slave SPI**: real boards usually have separate SS lines per
  device. The COR24-TB has one. Stay single-slave for now and add a
  bitmask `seln` register if/when the hardware grows more lines.
- **Clock stretching**: do any planned devices actually need it? If
  not, `slave_scl_pull` stays always-false and that code path is
  dormant until a device that needs it shows up.
- **Where device modules live**: a sibling `peripherals/` module under
  `src/` keeps them out of `cpu/` (since they're not CPU concerns) and
  parallels `assembler` / `loader` at the same level.
- **Serialization**: confirm the Web UI doesn't need to round-trip
  device state through serde. If it does, devices need an enum-based
  dispatch instead of `dyn I2cDevice`, which closes off the third-party
  extension story unless we expose a registration callback.
- **Address conflicts**: should `attach_i2c_device` reject a duplicate
  address, or allow last-writer-wins? Reject seems safer; bus
  collisions on real hardware are pathological.
- **Wildcard logger**: is `logger@*` (sees all addresses) useful, or
  confusing? Probably useful for debugging but should be opt-in and
  obvious in the log.
