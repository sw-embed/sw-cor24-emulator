# Extended Assembler: Compatibility with Reference cor24 Assembler

## Background

The COR24 emulator includes a Rust-based assembler that supports all standard
cor24 instructions plus several convenience extensions. The reference cor24
assembler (`as24`, running as `asld24-server` on queenbee:7412) does not
support these extensions.

We want hand-written `.s` examples in the Web UI and CLI to be **bug-compatible**
with the reference assembler — meaning they assemble correctly with `as24` as
shipped, regardless of whether our extensions are technically correct. If `as24`
doesn't support `la`, our examples must not use `la`, even though it's a real
hardware instruction. The goal is that a student can copy any example from the
Web UI and assemble it with the tools they actually have.

## Validation Results

Instruction-by-instruction encoding comparison shows **100% agreement** on all
85 instruction encodings that both assemblers support. The Rust assembler
produces identical bytes for every standard instruction.

One minor difference: `nop` — ours emits 1 byte (`0x00` = `add r0,r0`),
reference emits 3 bytes (`00 00 00`).

## Extensions Our Assembler Supports (Reference Does Not)

### 1. ~~`la` — Load Address~~ RESOLVED

**Update (2026-03-16):** Testing revealed `la` IS supported by the reference
assembler. The original test failure was due to hex immediates (`la r0,0xFF0000`
fails, but `la r0,-65536` works). The reference assembler supports `la` with
decimal immediates and with labels (including forward references).

`la` requires NO changes — just convert hex operands to decimal/negative
notation. Examples: `0xFF0000` → `-65536`, `0xFF0100` → `-65280`,
`0x0100` → `256`.

### 2. Inline Labels (`label: instruction`)

Our assembler allows `halt: bra halt` on one line. The reference requires
the label on its own line:

```
; Our syntax (extended):
halt: bra halt

; Reference syntax:
halt:
        bra halt
```

**Impact:** ALL 12 assembler examples use inline labels.

### 3. `#` Comments

Our assembler accepts both `;` and `#` as comment characters. The reference
only accepts `;`.

**Impact:** 1 file (`comments.s`).

### 4. Hex Immediates in Standard Instructions

`lc r0, 0x3F` works in our assembler. The reference requires decimal: `lc r0, 63`.
Note: hex in `la` operands is moot since `la` itself is unsupported.

**Impact:** 1 file (`echo.s`) uses hex in `lc`/`lcu` operands.

### 5. Other Extensions (unused in examples)

- `sxt`, `zxt` (sign/zero extend) — not used in any example
- `sub rX, imm` on GP registers — not used (only `sub sp, imm` which is standard)

## File-by-File Audit

### Assembler Examples (`src/examples/assembler/`)

Since `la` is supported by the reference assembler (with decimal operands),
the remaining incompatibilities are: inline labels, `#` comments, and hex
immediates.

| File | Inline labels | `#` comments | Hex immediates |
|---|---|---|---|
| `add.s` | 1 | - | 1 (in `la`) |
| `blink_led.s` | 1 | - | 1 (in `la`) |
| `button_echo.s` | 1 | - | 1 (in `la`) |
| `comments.s` | 1 | 2 | - |
| `countdown.s` | 1 | - | 1 (in `la`) |
| `echo.s` | - | - | 10 (5 in `la`, 5 in `lc`/`lcu`) |
| `fibonacci.s` | 2 | - | 1 (in `la`) |
| `memory_access.s` | 1 | - | 4 (in `la`) |
| `multiply.s` | 2 | - | 1 (in `la`) |
| `nested_calls.s` | 2 | - | - |
| `stack_variables.s` | 1 | - | 1 (in `la`) |
| `uart_hello.s` | 2 | - | 1 (in `la`) |

### Pipeline Examples (`src/examples/rust_pipeline/*.cor24.s`)

Compiler output — not intended to be reference-assembler-compatible.
Use hex immediates in `la` operands but are otherwise standard.

## Proposed Solution

### Principle: Bug-Compatible Examples

Examples must assemble with `as24` as shipped. Since `la` IS supported (just
not with hex operands), the fixes are all trivial text changes.

### Phase 1: Convert Hand-Written Examples (all trivial)

All 12 files need only these mechanical changes:

1. **Split inline labels** — `halt: bra halt` → label on its own line
2. **Replace `#` comments with `;`** — only `comments.s`
3. **Convert hex immediates to decimal** — in `la` operands and `lc`/`lcu`

Hex-to-decimal conversions needed:
- `0xFF0000` → `-65536` (LED I/O register)
- `0xFF0100` → `-65280` (UART data register)
- `0xFF0010` → `-65520` (interrupt enable register)
- `0x0100` → `256`
- `0x0200` → `512`
- `0x3F` → `63`, `0x21` → `33`, `0x61` → `97`, `0x7B` → `123`, `0xDF` → `223`

No structural changes needed. All examples can remain `.s` files compatible
with both assemblers.

### Phase 2 (optional): Preprocessor for .sx Files

If we want to keep hex-immediate syntax as a convenience, a simple text
preprocessor can expand `.sx` → `.s`:

```
Source (.sx)  →  Preprocessor  →  Standard .s  →  Assembler  →  Binary
```

**Single-pass macro expansions (no size changes):**

| Input | Output |
|---|---|
| `halt: bra halt` | `halt:\n        bra halt` |
| `# comment` | `; comment` |
| `lc r0, 0x3F` | `lc r0, 63` |
| `la r0, 0xFF0100` | `la r0, -65280` |

No instruction size changes, so branch offsets are unaffected.

### Phase 3 (optional): Assembler `--strict` Flag

Add a mode that rejects syntax `as24` wouldn't accept:
- Rejects inline labels, `#` comments, hex immediates
- Validates `.s` files are truly as24-compatible
- Use in CI to prevent regressions

### Additional Findings from Testing

- Reference assembler does NOT support `.byte` directive
- Reference assembler `nop` = 3 bytes (`00 00 00`), ours = 1 byte (`00`)
- `jal r1,(r2)` syntax matches between both assemblers
- `la` with forward label references works in the reference assembler
