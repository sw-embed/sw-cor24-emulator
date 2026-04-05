# COR24 Projects: History, Binaries, and Migration Guide

## History

The COR24 toolchain started as a single monolith: **cor24-rs**. It contained
the CPU emulator, assembler, web UI (Yew/WASM), debugger CLI, and
Rust-to-COR24 translation pipeline — all in one repository.

As the ecosystem grew (C compiler, P-code VM, Pascal runtime, linker), the
monolith was decomposed into focused projects:

- **sw-cor24-emulator** (this project) — emulator library + CLI binaries
- **sw-cor24-x-assembler** — standalone assembler library
- **sw-cor24-rust** — Rust/WASM/MSP430-to-COR24 translation pipeline
- **sw-cor24-x-tinyc** — C compiler targeting COR24
- **sw-cor24-pcode** — P-code assembler, linker, and trace interpreter
- **web-sw-cor24-assembler** — browser-based assembler + emulator UI

**cor24-rs** is the original monolith. It still exists but is being
superseded by the decomposed projects above. New development happens in the
`sw-*` projects.

## Project Map

### Libraries

| Library | Project | Purpose |
|---------|---------|---------|
| `cor24-emulator` | sw-cor24-emulator | CPU emulator, assembler, loader, challenges |
| `cor24-isa` | sw-cor24-emulator/isa | ISA definitions (opcodes, encoding, decode ROM) |
| `cor24-assembler` | sw-cor24-x-assembler | Standalone assembler (depends on cor24-emulator) |

### Binaries

| Binary | Source Project | Purpose | Status |
|--------|--------------|---------|--------|
| `cor24-emu` | **sw-cor24-emulator** | Headless emulator + assembler CLI | **Current** — use this |
| `cor24-dbg` | **sw-cor24-emulator** | Interactive GDB-like debugger | **Current** — use this |
| `cor24-run` | sw-cor24-rust | Headless emulator + assembler CLI | Legacy — predecessor to cor24-emu |
| `cor24-dbg` | cor24-rs | Interactive debugger | Superseded — replaced by sw-cor24-emulator |
| `wasm2cor24` | sw-cor24-rust | WASM-to-COR24 translator | Active (stays in sw-cor24-rust) |
| `msp430-to-cor24` | sw-cor24-rust | MSP430-to-COR24 translator | Active (stays in sw-cor24-rust) |
| `tc24r` | sw-cor24-x-tinyc | C-to-COR24 compiler | Active |
| `tc24r-pp` | sw-cor24-x-tinyc | C preprocessor | Active |
| `pasm` | sw-cor24-pcode | P-code assembler | Active |
| `plink` | sw-cor24-pcode | P-code linker | Active |
| `pv24t` | sw-cor24-pcode | P-code trace interpreter | Active |

## cor24-emu vs cor24-run

`cor24-emu` is the production replacement for `cor24-run`. They share the
same CLI interface — same flags, same behavior — so migration is a rename.

### What's the same

- `--run <file.s>` — assemble and execute
- `--load-binary <file>@<addr>` — load raw binary at address
- `--patch <addr>=<value>` — patch memory before execution
- `--terminal` / `--echo` — interactive UART bridge
- `--dump` — register + memory dump at halt
- `--trace <N>` — instruction trace on halt
- `--step` — single-step with trace output
- `-u` / `--uart-input` — send characters to UART RX
- `-n` / `--max-insns` — instruction limit
- `-s` / `--speed` — inter-instruction delay
- `--assemble` — assemble to binary + listing
- `--demo` — built-in LED counter demo
- `.p24` magic header auto-detection

### What's new in cor24-emu

- `-V` / `--version` — build provenance (git SHA, timestamp, hostname)
- `--dump-uart` — chronological UART transaction log showing input and
  output as coalesced groups:
  ```
  --- UART Transaction Log (6 entries) ---
    OUT:  "?"
     IN:  "abc!"
    OUT:  "ABC"
  ```

### What's different under the hood

- `cor24-emu` uses the `cor24-emulator` library directly (assembler is
  built-in), while `cor24-run` uses `cor24-assembler` + `cor24-emulator`
  as separate dependencies
- `cor24-emu` binary name clearly identifies its source project in
  `sw-install --list`

## cor24-dbg (this project) vs cor24-dbg (cor24-rs)

Both are the same GDB-like interactive debugger. The sw-cor24-emulator
version adds `-V`/`--version` support and is the actively maintained copy.
Installing from this project upgrades the cor24-rs version in place (same
binary name).

## Dependency Graph

```
cor24-emulator (this project, library)
├── cor24-isa (ISA definitions)
│
├── sw-cor24-x-assembler (standalone assembler library)
│   └── used by: sw-cor24-rust, web-sw-cor24-assembler
│
├── sw-cor24-rust (translation pipeline)
│   ├── binaries: wasm2cor24, msp430-to-cor24, cor24-run (legacy)
│   └── depends on: cor24-emulator, cor24-assembler
│
├── sw-cor24-x-tinyc (C compiler)
│   ├── binaries: tc24r, tc24r-pp
│   └── depends on: cor24-isa (via tc24r-emit-core)
│
├── web-sw-cor24-assembler (browser UI)
│   └── depends on: cor24-emulator, cor24-assembler
│
└── sw-cor24-pcode (P-code tools, self-contained)
    ├── binaries: pasm, plink, pv24t
    └── calls: cor24-run/cor24-emu binary for running COR24 programs
```

## When to Use Which

| Task | Use |
|------|-----|
| Run a `.s` assembly file | `cor24-emu --run prog.s` |
| Run with memory dump | `cor24-emu --run prog.s --dump --speed 0` |
| Interactive UART session | `cor24-emu --run repl.s --terminal` |
| Load guest binary (p24, forth) | `cor24-emu --run pvm.s --load-binary guest.p24@0x010000 --terminal` |
| Debug interactively (breakpoints, examine) | `cor24-dbg prog.lgo` |
| Compile Rust to COR24 | `msp430-to-cor24` (from sw-cor24-rust) |
| Compile C to COR24 | `tc24r` (from sw-cor24-x-tinyc) |
| Assemble P-code | `pasm` (from sw-cor24-pcode) |
| Link P-code | `plink` (from sw-cor24-pcode) |
| Use emulator as a Rust library | `cor24-emulator` crate (this project) |

## Projects That Need to Migrate

### Must migrate (call `cor24-run` binary)

These projects invoke `cor24-run` in scripts or tests and should switch to
`cor24-emu`:

| Project | Where | Migration |
|---------|-------|-----------|
| **pv24a** | `demo.sh` | Change `cor24-run` to `cor24-emu` |
| **sw-cor24-pcode** | `demo.sh` | Change `cor24-run` to `cor24-emu` |
| **sw-cor24-x-tinyc** | `demos/run-demo*.sh`, `scripts/run-chibicc-test.sh`, `scripts/run-subset-tests.sh` | Change `cor24-run` to `cor24-emu` |
| **sw-cor24-rust** | `demo-*.sh` scripts (8+ files) | Change `cor24-run` to `cor24-emu`, then remove `cor24-run` binary |

### Should migrate (depend on cor24-emulator library)

These already depend on `cor24-emulator` from this project — no change
needed unless they also depend on `cor24-rs`:

| Project | Cargo.toml dep | Status |
|---------|---------------|--------|
| **sw-cor24-rust** | `cor24-emulator = { path = "../sw-cor24-emulator" }` | Already migrated |
| **sw-cor24-x-assembler** | `cor24-emulator = { path = "../sw-cor24-emulator" }` | Already migrated |
| **web-sw-cor24-assembler** | `cor24-emulator = { path = "../sw-cor24-emulator" }` | Already migrated |
| **sw-cor24-x-tinyc** | `cor24-isa = { path = "../sw-cor24-emulator/isa" }` | Already migrated |

### No migration needed

| Project | Reason |
|---------|--------|
| **cor24-rs** | Original monolith, kept for reference; web UI may stay here |
| **pa24r** | Placeholder, not actively using cor24-run yet |
| **pl24r** | Pipeline scripts reference `pasm`/`plink`, not cor24-run |

## Migration Checklist

For each project migrating from `cor24-run` to `cor24-emu`:

1. Search for `cor24-run` in all scripts: `grep -r cor24-run scripts/ demos/ *.sh`
2. Replace `cor24-run` with `cor24-emu` (same flags, drop-in replacement)
3. Run the project's tests/demos to verify identical behavior
4. Optionally use `--dump-uart` where UART debugging was previously manual
