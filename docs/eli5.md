# ELI5: What's in This Project?

This project is a set of tools for working with the COR24 CPU — a small
24-bit processor used for teaching embedded systems. You can write
programs for it, assemble them into machine code, and run them in an
emulator — all without needing the physical hardware.

There are **two ways** to write programs for the COR24:

1. **Write assembly by hand** — you write COR24 instructions directly
2. **Write Rust** — a compiler translates your Rust code into COR24 assembly automatically

## The Binaries

### Web Emulator (the main app)

**What:** A website that runs in your browser.
**Built by:** `./build.sh` (or `./serve.sh` for development).
**Output:** `pages/` directory, deployed to GitHub Pages.

This is the primary way to use the project. It has two tabs:
- **Assembler tab** — paste or write COR24 assembly, click Assemble, click Run
- **Rust tab** — browse pre-built Rust examples showing each compilation stage

There is no standalone binary for this — it compiles to WebAssembly and
runs in the browser.

### `cor24-dbg` — CLI Debugger

**What:** A GDB-like command-line debugger for COR24 programs.
**Where:** `cli/` directory.
**Build:** `cargo build -p cor24-cli`
**Binary:** `target/debug/cor24-dbg`

**When to use:** When you want to debug a COR24 program from the terminal
instead of the web UI. Supports breakpoints, single-stepping, register
inspection, memory dumps, and UART output capture.

```bash
# Load a pre-assembled .lgo file and debug it
./target/debug/cor24-dbg tests/programs/hello_world.lgo

# Commands inside the debugger:
#   run 1000     — run 1000 instructions
#   step         — single step
#   regs         — show registers
#   uart         — show UART output
#   break 0x20   — set breakpoint at address
#   quit         — exit
```

**Demo scripts:** `scripts/demo-cli-*.sh` build and run various examples
through the debugger.

### `msp430-to-cor24` — MSP430 → COR24 Translator

**What:** Translates MSP430 assembly into COR24 assembly.
**Where:** `rust-to-cor24/` directory.
**Build:** `cargo build --release` (in `rust-to-cor24/`)
**Binary:** `target/release/msp430-to-cor24`

**When to use:** This is the middle step of the Rust pipeline. You don't
usually call it directly — the demo scripts handle it. But if you're
compiling your own Rust code for COR24, you'd use it like:

```bash
# Rust → MSP430 (done by rustc)
rustup run nightly cargo rustc --target msp430-none-elf \
    -Z build-std=core --release -- --emit asm

# MSP430 → COR24 (this tool)
msp430-to-cor24 my_program.msp430.s -o my_program.cor24.s
```

**Why MSP430?** There's no `rustc` backend for COR24, but MSP430 is a
similar 16-bit architecture that Rust can already target. The translator
maps MSP430 registers and instructions to their COR24 equivalents.

### `cor24-emu` — Headless Emulator

**What:** Runs a pre-built `.lgo` in the emulator and dumps registers,
memory, and UART output. Assembly (`.s` → `.lgo`) is `cor24-asm`'s
job; this binary is a pure runtime consumer.
**Build:** `cargo build --release`
**Binary:** `target/release/cor24-emu`

**When to use:** when you want to run a COR24 assembly program
non-interactively and see the results. Used by the demo scripts.

```bash
# Assemble, then run with register/memory dump
cor24-asm program.cor24.s -o program.lgo
cor24-emu --lgo program.lgo --dump --speed 0

# With a time limit (seconds) and instruction limit
cor24-emu --lgo program.lgo --dump --speed 0 --time 5 -n 100000

# Feed UART input (for interactive programs like echo)
cor24-asm echo.cor24.s -o echo.lgo
cor24-emu --lgo echo.lgo --dump --speed 0 --uart-input 'abc\x21'
```

### `wasm2cor24` — Experimental WASM → COR24 (unused)

**What:** An experimental translator from WebAssembly to COR24.
**Where:** `rust-to-cor24/` directory.
**Status:** Early prototype, not used by any demos. The MSP430 path
(`msp430-to-cor24`) is the active pipeline.

## How They Fit Together

```
                        ┌─────────────────────────────────────┐
  Hand-written          │         Web Emulator (browser)      │
  COR24 assembly ──────►│  Assembler tab: edit → assemble → run│
                        │  Rust tab: browse pre-built examples │
                        └─────────────────────────────────────┘

  Hand-written          ┌──────────────┐
  COR24 assembly ──────►│  cor24-dbg   │  (CLI debugger)
  (.lgo files)          └──────────────┘

                        ┌──────────┐    ┌─────────────────┐    ┌──────────┐
  Rust source ─────────►│  rustc   │───►│ msp430-to-cor24 │───►│ cor24-run│
  (.rs)        (MSP430) │(compiler)│    │  (translator)   │    │(emulator)│
                        └──────────┘    └─────────────────┘    └──────────┘
```

## Quick Start

```bash
# Run the web emulator locally
./serve.sh
# Open http://localhost:7401/cor24-rs/

# Run an assembler demo from CLI
scripts/demo-cli-hello-world.sh

# Run a Rust pipeline demo from CLI
rust-to-cor24/demos/run-demo.sh demo_add --skip-compile
```

## See Also

- [Assembler Examples](assembler-examples.md) — hand-written COR24 assembly programs
- [Rust Pipeline Demos](rust-pipeline-demos.md) — Rust programs compiled to COR24
- [rust-to-cor24/README.md](../rust-to-cor24/README.md) — translator internals
