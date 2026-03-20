# COR24 Assembly Tutorial

## What is the COR24?

The COR24 is a 24-bit RISC processor designed by Luther Johnson for the
[MakerLisp](https://makerlisp.com) project. "COR" stands for C-Oriented RISC.
It runs as a soft CPU on Lattice MachXO FPGAs and is used to teach embedded
systems concepts.

This emulator lets you write, assemble, and step through COR24 assembly code
in your browser. You can also explore compilation pipelines from C and Rust
down to COR24 machine code.

## CPU Overview

- **24-bit architecture** — all registers and addresses are 24 bits wide
- **8 registers** — 3 general-purpose, 5 special-purpose
- **Variable-length instructions** — 1, 2, or 4 bytes
- **Single condition flag (C)** — set by compare instructions, tested by branches
- **16 MB address space** — SRAM, stack (EBR), and memory-mapped I/O

## Registers

| Register | Name | Purpose |
|----------|------|---------|
| r0 | — | General purpose |
| r1 | — | General purpose (also return address for jal) |
| r2 | — | General purpose |
| fp | — | Frame pointer |
| sp | — | Stack pointer (init: 0xFEEC00) |
| z | — | Always zero; used in compares (ceq r0,z) |
| iv | — | Interrupt vector (ISR address) |
| ir | — | Interrupt return address |

Only r0, r1, r2 can be used as destinations for most instructions.
The sp grows downward (push decrements, pop increments).

## Getting Started

Try loading the **Comments** example from the Examples dialog. It shows
the basic workflow:

1. **Write** assembly code in the editor (or load an example)
2. **Assemble** — click the Assemble button to convert to machine code
3. **Step** — execute one instruction at a time and watch registers change
4. **Run** — execute continuously (click Stop to pause)

Register values that just changed are highlighted.

## Loading Constants

```
lc  r0, 42      ; load signed 8-bit (-128 to 127)
lcu r0, 200     ; load unsigned 8-bit (0 to 255)
la  r0, 1000    ; load 24-bit value (any address)
```

`lc` sign-extends the byte: `lc r0, -1` sets r0 to 0xFFFFFF.
`lcu` zero-extends: `lcu r0, 200` sets r0 to 0x0000C8.
`la` loads a full 24-bit immediate (4-byte instruction).

## Arithmetic

```
add r0, r1      ; r0 = r0 + r1
add r0, 5       ; r0 = r0 + 5 (signed 8-bit immediate)
sub r0, r1      ; r0 = r0 - r1
mul r0, r1      ; r0 = r0 * r1
sub sp, 12      ; allocate 12 bytes on stack (4-byte instruction)
```

All results are masked to 24 bits (0-16777215). Overflow wraps silently.

## Logic and Shifts

```
and r0, r1      ; r0 = r0 AND r1
or  r0, r1      ; r0 = r0 OR r1
xor r0, r1      ; r0 = r0 XOR r1
shl r0, r1      ; r0 = r0 << r1 (shift left)
srl r0, r1      ; r0 = r0 >> r1 (shift right, zero fill)
sra r0, r1      ; r0 = r0 >> r1 (shift right, sign fill)
```

## Compare and Branch

COR24 has one condition flag **C**. Compare instructions set it; branch
instructions test it.

```
ceq r0, r1      ; C = (r0 == r1)
clu r0, r1      ; C = (r0 < r1)  unsigned
cls r0, r1      ; C = (r0 < r1)  signed
ceq r0, z       ; C = (r0 == 0)  compare with zero register
```

```
bra label       ; branch always (unconditional)
brt label       ; branch if C = true
brf label       ; branch if C = false
```

Branches are PC-relative with a signed 8-bit offset.

### Example: loop while r0 < 10

```
        lc  r0, 0
loop:
        add r0, 1
        lc  r1, 10
        clu r0, r1      ; C = (r0 < 10)
        brt loop         ; keep going while true
```

### Halting

COR24 has no halt instruction. The idiom is a branch-to-self:

```
halt:
        bra halt         ; emulator detects this as halt
```

## Register Moves

```
mov r0, r1      ; r0 = r1
mov r0, c       ; r0 = condition flag (0 or 1)
mov fp, sp      ; save stack pointer to frame pointer
mov sp, fp      ; restore stack pointer from frame pointer
mov iv, r0      ; set interrupt vector
```

## Memory Access

COR24 uses base+offset addressing. The offset is a signed 8-bit value.

```
; Store
sb  r0, 0(r1)   ; store byte at address r1+0
sw  r0, 0(r1)   ; store word (3 bytes) at address r1+0

; Load
lb  r0, 0(r1)   ; load byte (sign-extended to 24 bits)
lbu r0, 0(r1)   ; load byte (zero-extended)
lw  r0, 0(r1)   ; load word (3 bytes)
```

Valid base registers: r0, r1, r2, fp. (Not sp — use fp instead.)

### Example: store 42 to address 256

```
        lc  r0, 42
        la  r1, 256
        sb  r0, 0(r1)    ; mem[256] = 42
```

## Stack Operations

The stack lives in EBR memory, growing downward from 0xFEEC00.

```
push r0         ; sp -= 3; mem[sp] = r0
pop  r0         ; r0 = mem[sp]; sp += 3
push fp         ; save frame pointer
pop  fp         ; restore frame pointer
```

Each push/pop moves sp by 3 bytes (one 24-bit word).

## Calling Functions

Use `jal` (jump and link) for function calls. It saves the return
address in the link register (first operand) and jumps to the target.

```
; Caller:
        la  r2, my_func  ; load function address
        jal r1, (r2)     ; r1 = return addr, jump to r2

; Callee:
my_func:
        push r1           ; save return address
        ; ... function body ...
        pop  r1           ; restore return address
        jmp  (r1)         ; return to caller
```

### Calling convention

```
; Prologue
        push fp
        push r1           ; save return address
        mov  fp, sp

; Body (use r0-r2 freely, push/pop to spill)

; Epilogue
        mov  sp, fp
        pop  r1
        pop  fp
        jmp  (r1)
```

## Memory-Mapped I/O

| Address | Device | Access |
|---------|--------|--------|
| 0xFF0000 | LED D2 (bit 0) / Button S2 (bit 0) | Read: button state. Write: LED state |
| 0xFF0010 | Interrupt enable | Write bit 0 = enable UART RX interrupt |
| 0xFF0100 | UART data | Read: RX byte. Write: TX byte |
| 0xFF0101 | UART status | Bit 7 = TX busy. Bit 1 = RX data ready |

### Example: blink LED

```
        la  r1, -65536   ; 0xFF0000
loop:
        lc  r0, 1
        sb  r0, 0(r1)    ; LED on
        lc  r0, 0
        sb  r0, 0(r1)    ; LED off
        bra loop
```

### Example: print a character to UART

```
        la  r1, -65280   ; 0xFF0100 (UART data)
.wait:
        lb  r2, 1(r1)    ; read UART status
        lcu r0, 128
        and r2, r0       ; isolate bit 7 (TX busy)
        ceq r2, z
        brf .wait         ; spin while busy
        lc  r0, 65       ; 'A'
        sb  r0, 0(r1)    ; transmit
```

## Extension Instructions

```
sxt r0          ; sign-extend byte: bits 7..23 = bit 7
zxt r0          ; zero-extend byte: bits 8..23 = 0
```

Useful after loading a byte value that you want to treat as signed
or need to mask to 8 bits.

## Interrupts

COR24 supports a single interrupt (UART RX). To use interrupts:

1. Set the interrupt vector: `mov iv, r0` (where r0 = ISR address)
2. Enable interrupts by writing 1 to 0xFF0010
3. When UART data arrives, the CPU jumps to the ISR address
4. Return from ISR with `jmp (ir)`

The ISR must save and restore all registers it uses (push/pop).
Reading the UART data register acknowledges the interrupt.

## Instruction Encoding

Instructions are 1, 2, or 4 bytes:

| Size | Format | Examples |
|------|--------|----------|
| 1 byte | opcode + registers | add, sub, mul, and, or, xor, mov, push, pop, jmp, jal, ceq, clu, cls, shl, srl, sra, sxt, zxt |
| 2 bytes | opcode + 8-bit immediate | lc, lcu, add imm, bra, brt, brf, lb, lbu, lw, sb, sw |
| 4 bytes | opcode + 24-bit immediate | la, sub sp |

The first byte encodes the opcode and register operands via a decode ROM
extracted from the hardware Verilog. The remaining bytes (if any) are the
immediate value in little-endian byte order.

## Assembly Syntax

```
; Comments start with semicolon
label:                   ; labels end with colon
        lc  r0, 42      ; instruction with operands
.local:                  ; local labels start with dot
        bra .local       ; branch to local label
```

Labels can be on their own line or before an instruction. The reference
assembler (as24) requires labels on their own line.

## Tips

- **Step through code** to understand what each instruction does
- **Watch registers** — changed values are highlighted in the debug panel
- **Expand the Instruction Trace** to see a history of recent instructions
- **Use the ISA Reference** (sidebar button) for quick instruction lookup
- Load the **Assert** example to see how to validate results
- Load the **Loop Trace** example to practice Run/Stop/Trace workflow
- All assembler examples are editable — experiment freely!
