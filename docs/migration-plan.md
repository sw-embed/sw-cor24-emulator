# Migration Plan: cor24-rs / sw-cor24-rust → sw-cor24-emulator

This document describes how to transition dependent projects from using
`cor24-rs` and `sw-cor24-rust` to `sw-cor24-emulator` as the single source
of COR24 emulator functionality.

## Current State

### Binary Landscape

| Project | Binary | Role | Version flag |
|---------|--------|------|-------------|
| cor24-rs | `cor24-dbg` | Interactive debugger (GDB-like REPL) | No |
| sw-cor24-rust | `cor24-run` | Headless emulator (batch execution) | Yes (`-V`) |
| sw-cor24-rust | `wasm2cor24` | WASM→COR24 translator | No |
| sw-cor24-rust | `msp430-to-cor24` | MSP430→COR24 translator | No |
| **sw-cor24-emulator** | `cor24-dbg` | Interactive debugger (refactored) | **No (add)** |

### Library Dependencies

```
sw-cor24-rust/Cargo.toml:
  cor24-emulator = { path = "../sw-cor24-emulator" }   # already points here

pv24a:
  calls `cor24-run` binary (from sw-cor24-rust, expects in PATH)

pa24r:
  references `cor24-run` in docs (not yet active)
```

### What Lives Where

- **cor24-rs**: Original monolith. Contains emulator library, web UI (Yew/WASM),
  debugger CLI, Rust-to-COR24 pipeline. Being decomposed.
- **sw-cor24-rust**: Rust-to-COR24 translation pipeline (`wasm2cor24`,
  `msp430-to-cor24`) plus headless runner (`cor24-run`). Already depends on
  sw-cor24-emulator for emulator library.
- **sw-cor24-emulator** (this project): Refactored emulator library + debugger
  CLI. Should become the canonical source for all emulator binaries.

## Migration Plan

### Phase 1: Add `cor24-emu` binary to sw-cor24-emulator

Add a new binary `cor24-emu` to this project's `cli/` workspace member.
This replaces `cor24-run` from sw-cor24-rust with a distinct name that
identifies its source project.

**Binary name:** `cor24-emu` (not `cor24-run`, to avoid ambiguity during
transition and clearly identify the source in `sw-install --list`).

**Features to port from `cor24-run`:**
- `--run <file.s>` — assemble and execute
- `--run <file.lgo>` — load LGO and execute
- `--load-binary <file>@<addr>` — load raw binary at address
- `--patch <addr>=<value>` — patch memory before execution
- `-u <string>` / `--uart-input <string>` — send UART input
- `-n <count>` / `--max-insns <count>` — instruction limit
- `-s <ms>` / `--speed <ms>` — inter-instruction delay
- `--dump` — register + memory dump at halt
- `--dump-uart` — show UART transaction log (new feature)
- `-V` / `--version` — version and build provenance
- `-h` / `--help` — short help
- `--help` — long help with examples

**Version output format** (matches sw-cor24-rust / sw-cli-tools convention):
```
cor24-emu 0.1.0
Copyright (c) 2026 Michael A Wright
License: MIT
Repository: https://github.com/sw-embed/sw-cor24-emulator

Build Information:
  Host: mikes-macbook.local
  Commit: a1b2c3d
  Timestamp: 2026-04-05T10:44:22Z
```

**Implementation:**
- Add `cli/build.rs` to embed `VERGEN_GIT_SHA_SHORT`, `VERGEN_BUILD_TIMESTAMP`,
  `VERGEN_BUILD_HOST` (same pattern as root `build.rs`)
- Add `[[bin]] name = "cor24-emu"` to `cli/Cargo.toml`
- Port `cor24-run` logic from `sw-cor24-rust/src/run.rs`
- Manual arg parsing (no clap), matching ecosystem convention

### Phase 2: Add `--dump-uart` flag

Gate the new UART transaction log behind `--dump-uart` so existing scripts
that parse `--dump` output don't break. The flag prints the coalesced
chronological UART log after the I/O state section:

```
=== I/O FF0000-FFFFFF (64 KB, memory-mapped peripherals) ===
  LED D2:  0x01  off
  BTN S2:  0x01  released
  FF0010 IntEn:  0x00  UART RX IRQ: disabled
  FF0100 UART:   status=0x02  RX ready: no  CTS: yes  TX busy: no
  UART TX log:   "Hello, world!\n"
  --- UART Transaction Log (12 entries) ---
   IN:  "hello\n"
  OUT:  "HELLO\n"
```

### Phase 3: Update dependent projects

#### sw-cor24-rust
- Remove `cor24-run` binary (keep `wasm2cor24`, `msp430-to-cor24`)
- Update demo scripts to use `cor24-emu` instead of `cor24-run`
- Library dependency already points to sw-cor24-emulator

#### pv24a
- Update `demo.sh` to call `cor24-emu` instead of `cor24-run`
- Same CLI interface, just different binary name
- Install via `sw-install` from sw-cor24-emulator

#### pa24r
- Update documentation references from `cor24-run` to `cor24-emu`

#### cor24-rs
- Remove `cli/` workspace member (debugger moves here)
- Keep web UI code (Yew/WASM) — it uses the library, not the CLI
- Point library dependency to sw-cor24-emulator

### Phase 4: Install via sw-install

```bash
# Install cor24-emu and cor24-dbg from this project
cd ~/github/sw-embed/sw-cor24-emulator
sw-install --release

# Verify provenance
sw-install --list
# cor24-emu   0.1.0  a1b2c3d  2026-04-05  sw-cor24-emulator
# cor24-dbg   0.1.0  a1b2c3d  2026-04-05  sw-cor24-emulator
```

## Regression Prevention

### Before switching any dependent project:

1. **Run this project's tests:** `cargo test` (225 tests covering assembler,
   CPU, emulator, UART log, echo programs, sieve, etc.)

2. **Run sw-cor24-rust's tests** after pointing at this library:
   `cargo test` in sw-cor24-rust (already depends on us)

3. **Run pv24a demo suite** with `cor24-emu` in PATH:
   `cd ~/github/sw-vibe-coding/pv24a && ./demo.sh`

4. **Diff test:** Run the same program with both `cor24-run` and `cor24-emu`,
   diff the `--dump` output to verify identical behavior:
   ```bash
   cor24-run --run test.s --dump > /tmp/old.txt
   cor24-emu --run test.s --dump > /tmp/new.txt
   diff /tmp/old.txt /tmp/new.txt
   ```

### API compatibility

The `EmulatorCore` API is identical between cor24-rs and sw-cor24-emulator.
New additions (e.g., `uart_log()`, `format_uart_log()`) are additive — no
existing methods changed signatures or behavior.

### Test coverage map

| Area | Tests | Source |
|------|-------|--------|
| Assembler | 41 | `src/assembler.rs` |
| CPU decode/encode | 5 | `src/cpu/decode_rom.rs` |
| CPU execution | 67 | `src/cpu/executor.rs` |
| CPU state + I/O | 33 | `src/cpu/state.rs` (includes 13 UART log tests) |
| Emulator core | 10 | `src/emulator.rs` |
| Loader | 10 | `src/loader.rs` |
| Integration | 21 | `tests/integration_tests.rs` |
| **Total** | **225** | |
