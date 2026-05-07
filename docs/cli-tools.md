# COR24 CLI Tools

## Overview

The COR24 toolchain consists of three CLI tools. They are written in
Rust and split assembly from execution: `cor24-asm` produces artifacts,
`cor24-emu` consumes them.

```
Rust source (.rs)
    ↓ rustc --target msp430-none-elf --emit asm
MSP430 assembly (.msp430.s)
    ↓ msp430-to-cor24 (translator)
COR24 assembly (.cor24.s)
    ↓ cor24-asm   (sw-cor24-x-assembler)
.lgo / .bin / .lst
    ↓ cor24-emu --lgo / --load-binary
Execution + final state
```

## cor24-emu — Batch runner

Loads a pre-built `.lgo` (or raw bytes via `--load-binary`) and runs it.
The emulator does not assemble — point it at output from `cor24-asm`
or another producer of the `.lgo` format.

```bash
# Two-step: assemble, then run
cor24-asm fibonacci.s -o fibonacci.lgo
cor24-emu --lgo fibonacci.lgo --dump

# With instruction trace (last 50 instructions)
cor24-emu --lgo fibonacci.lgo --dump --trace 50

# With UART input (for interactive programs like Echo)
cor24-asm echo.s -o echo.lgo
cor24-emu --lgo echo.lgo --uart-input 'abc!' --dump

# Timed execution with speed limit
cor24-asm blink_led.s -o blink_led.lgo
cor24-emu --lgo blink_led.lgo --speed 100000 --time 5

# Step mode: print each instruction as it executes
cor24-emu --lgo fibonacci.lgo --step

# Run a built-in demo (no .s/.lgo input needed)
cor24-emu --demo --speed 100000 --time 10

# Load a guest binary into memory at a fixed address
cor24-asm pvm.s -o pvm.lgo
cor24-emu --lgo pvm.lgo --load-binary hello.p24@0x010000 --terminal

# Multiple binaries at different addresses
cor24-asm loader.s -o loader.lgo
cor24-emu --lgo loader.lgo \
          --load-binary code.bin@0x010000 \
          --load-binary data.bin@0x020000

# Interactive terminal mode (stdin/stdout bridged to UART)
cor24-asm repl.s -o repl.lgo
cor24-emu --lgo repl.lgo --terminal --echo --speed 0
```

### Options

| Flag | Description |
|------|-------------|
| `--lgo <file.lgo>` | Load a pre-built `.lgo` and run it |
| `--demo` | Run the built-in LED counter demo |
| `--load-binary <file>@<addr>` | Load raw bytes into memory at address (repeatable) |
| `--patch <addr>=<value>` | Write 24-bit value to memory after loading |
| `--entry <label\|addr>` | Set entry point |
| `--dump` | Dump CPU state, I/O, and non-zero memory after halt |
| `--dump-uart` | Show UART transaction log (chronological IN/OUT) |
| `--trace <N>` | Show last N instructions on halt/timeout (default: 50) |
| `--step` | Print each instruction as it executes |
| `--speed <N>` | Instructions per second (0 = unlimited) |
| `--time <secs>` | Time limit in seconds |
| `--max-instructions <N>` | Stop after N instructions (-1 = no limit) |
| `--uart-input <str>` | Send characters to UART RX (supports \n, \x21) |
| `--uart-file <path>` | Read file contents into UART RX buffer |
| `--uart-never-ready` | UART TX stays busy forever (test polling discipline) |
| `--terminal` | Bridge stdin/stdout to UART (interactive mode) |
| `--echo` | Local echo in terminal mode |
| `--switch <on\|off>` | Set button S2 state (default: off/released) |
| `--stack-kilobytes <3\|8>` | EBR stack size (default: 3, max: 8) |
| `--guard-jumps` | Halt if PC leaves the code region |
| `--code-end <addr>` | Upper bound for `--guard-jumps` |
| `--canary <addr>[=val]` | Halt if memory at `addr` changes |
| `--watch-range <lo> <hi>` | Halt if any byte in `[lo, hi]` changes |
| `--i2c-device <spec>` | Attach a virtual I2C device — repeatable. See below |
| `--dump-i2c` | Print the chronological I2C transaction log at end of run |
| `--quiet, -q` | UART TX as plain text on stdout; logs to stderr |
| `-h` | Short help |
| `--help` | Extended help with AI agent guidance |
| `-V, --version` | Version, copyright, license, build info |

Assembling `.s` to `.lgo` is the job of [`cor24-asm`](https://github.com/softwarewrighter/sw-cor24-x-assembler).
This emulator binary does not include an assembler.

### Loading guest binaries

The `--load-binary <file>@<addr>` flag loads raw bytes from a file into
emulator memory at a specified address. Loading happens after the
`.lgo` (if any) is loaded but before execution begins. This is useful
for VMs (p-code, Forth, Lisp) that need guest programs pre-loaded in memory.

Address formats: `0x010000` (hex prefix), `010000h` (hex suffix), `65536` (decimal).

The flag is repeatable — use multiple `--load-binary` flags to load
code and data segments at different addresses.

```bash
# P-code VM pipeline: assemble pvm.s, load guest .p24, run with UART I/O
cor24-asm pvm.s -o pvm.lgo
cor24-emu --lgo pvm.lgo --load-binary hello.p24@0x010000 \
          --terminal --speed 0 -n 50000000
```

### Attaching virtual I2C devices

`--i2c-device <spec>` registers a virtual chip on the emulator's
bit-banged I2C bus (MMIO at `0xFF0020` SCL / `0xFF0021` SDA). The flag
is repeatable; each spec gets its own 7-bit address. Recognised devices:

| Spec | Behaviour |
|------|-----------|
| `add1@<addr>[?wrap=<n>]` | Universal +1 test slave: write a byte to seed `last`, each subsequent read returns `last = (last + 1) % wrap` (default wrap = 256). |
| `tmp101@<addr>[?temp=<f>][?config=<n>]` | TI TMP101 temperature sensor: configure starting temperature in °C and starting config-register byte. Resolution honoured at read time. |

Address formats accept `0x4A` / `4A` / `74` (decimal). Example:

```bash
# Run the canonical TMP101 demo against a virtual sensor at 25°C, dump the bus log
cor24-emu --lgo examples/i2c/tmp101/tmp101.lgo \
          --i2c-device tmp101@0x4A?temp=25.0 \
          -n 200000 --quiet --dump-i2c
```

`--dump-i2c` renders one line per bus event with the CPU instruction
count as a timestamp, e.g.:

```
--- I2C Transaction Log (12 entries) ---
  I2C: i=      390  START
  I2C: i=     4062  ADDR 0x4A WR ACK
  I2C: i=     8329  WR   0x4A 0x01 ACK
  I2C: i=    26957  ADDR 0x4A RD ACK
  I2C: i=    31316  RD   0x4A 0x19
  ...
```

The runtime API behind these flags is in `cor24_emulator::peripherals::i2c`
(`build_i2c_device`, `EmulatorCore::attach_i2c_device`,
`EmulatorCore::format_i2c_log`); see `examples/web_surface_smoke.rs`
for a worked example of the full surface a Web UI is expected to call.

## cor24-dbg — Interactive debugger

GDB-like command-line debugger with breakpoints, memory inspection,
and UART I/O. Loads `.lgo` files (MakerLisp's "load and go" format).

```bash
cor24-dbg program.lgo
cor24-dbg --entry 0x93 program.lgo
```

### Commands

| Command | Description |
|---------|-------------|
| `r, run [N]` | Run N instructions (default 100M) |
| `s, step [N]` | Single step N instructions |
| `n, next` | Step over (skip jal calls) |
| `c, continue` | Continue from breakpoint |
| `b, break <addr>` | Set breakpoint |
| `d, delete <N\|all>` | Delete breakpoint(s) |
| `i, info [r\|b\|t]` | Show registers, breakpoints, or trace |
| `t, trace [N]` | Show last N traced instructions |
| `x [/N] <addr>` | Examine N bytes at address |
| `p, print <reg\|addr>` | Print register or memory |
| `disas [addr] [N]` | Disassemble N instructions |
| `uart` | Show UART output buffer |
| `uart send <val>` | Send byte to UART RX |
| `led` | Show LED/button state |
| `button [press\|release]` | Control button S2 |
| `reset` | Reset CPU |
| `q, quit` | Exit |

## msp430-to-cor24 — Translator

Translates MSP430 assembly (from `rustc`) to COR24 assembly. This is
a source-to-source translator — `.msp430.s` text in, `.cor24.s` text out.
No binary files are involved.

### Direct translation (two files)

```bash
# Step 1: Translate MSP430 .s → COR24 .s (writes output to file)
msp430-to-cor24 demo.msp430.s -o demo.cor24.s --entry start

# Step 2: Assemble + run the COR24 .s file
cor24-asm demo.cor24.s -o demo.cor24.lgo
cor24-emu --lgo demo.cor24.lgo --dump
```

The translator reads the MSP430 `.s` file, writes a COR24 `.s` file.
The `--entry` flag specifies which function is the entry point (default:
`start`). The translator generates a reset vector that jumps to it.

Without `-o`, the COR24 assembly is printed to stdout.

### Compile mode (from Rust source)

```bash
# One command: compile Rust → MSP430 → COR24 (prints to stdout)
msp430-to-cor24 --compile ./my-project --entry start
```

This runs `rustc --target msp430-none-elf --emit asm` inside the
project directory, finds the generated `.s` file in `target/`, and
translates it to COR24 assembly.

### Full pipeline step by step

```bash
# Step 1: Rust → MSP430 assembly
cd rust-to-cor24/demos/demo_fibonacci
rustup run nightly cargo rustc \
    --target msp430-none-elf \
    -Z build-std=core --release \
    -- --emit asm

# The .s file lands in target/msp430-none-elf/release/deps/*.s
cp target/msp430-none-elf/release/deps/*.s demo_fibonacci.msp430.s

# Step 2: MSP430 → COR24 assembly (text file to text file)
msp430-to-cor24 demo_fibonacci.msp430.s -o demo_fibonacci.cor24.s

# Step 3: Assemble COR24 .s -> .lgo
cor24-asm demo_fibonacci.cor24.s -o demo_fibonacci.lgo

# Step 4: Run in emulator
cor24-emu --lgo demo_fibonacci.lgo --dump --trace 50
```

### Intermediate files

All intermediate files are human-readable text:

```
src/lib.rs                    ← Rust source (you write this)
demo_fibonacci.msp430.s       ← MSP430 assembly (rustc produces this)
demo_fibonacci.cor24.s        ← COR24 assembly (translator produces this)
demo_fibonacci.lgo            ← Load-and-go text (cor24-asm produces this)
```

### What the translator does

- Maps MSP430 registers (r4-r15) to COR24 registers (r0-r2) + stack spill slots
- Translates MSP430 instructions to COR24 equivalents
- Remaps MSP430 I/O addresses to COR24 memory-mapped I/O
- Passes through `@cor24:` asm comments as literal COR24 instructions
- Generates reset vector prologue (`mov fp,sp` + `la r0,start` + `jmp (r0)`)

### Pipeline demos

Pre-built demos in `rust-to-cor24/demos/`:

```bash
cd rust-to-cor24/demos
bash generate-all.sh        # Compile + translate + run all 13 demos
bash demo_fibonacci/run.sh  # Run one specific demo
```

Each demo directory contains all intermediate files after running:
```
demo_fibonacci/
    src/lib.rs                    ← Rust source
    demo_fibonacci.msp430.s       ← MSP430 assembly from rustc
    demo_fibonacci.cor24.s        ← COR24 assembly from translator
    demo_fibonacci.log            ← Emulator output (registers, memory)
```

## File Formats

| Extension | Format | Description |
|-----------|--------|-------------|
| `.s` | Text | COR24 assembly source (as24-compatible) |
| `.cor24.s` | Text | COR24 assembly from translator pipeline |
| `.msp430.s` | Text | MSP430 assembly from rustc |
| `.lgo` | Text | MakerLisp's "load and go" monitor format |
| `.rs` | Text | Rust source |

There is no binary object file format. The assembler produces bytes
directly in memory — no linking step, no ELF headers, no relocations.
COR24 programs are flat: code starts at address 0, the reset vector
is the first few instructions.

## Assembly and Loading

The assembler is a two-pass assembler built into `cor24-run` and the
Web UI. It produces a byte array that is copied directly into the
emulator's 1 MB SRAM at address 0:

```rust
let result = assembler.assemble(source);  // → Vec<u8>
for (addr, byte) in result.bytes.iter().enumerate() {
    cpu.memory[addr] = *byte;             // load at address 0
}
cpu.pc = 0;                                // start executing
```

No separate linking or loading step. The assembler resolves all labels
internally using two passes (first pass collects label addresses,
second pass emits bytes with resolved references).
