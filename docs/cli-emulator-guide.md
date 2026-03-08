# COR24 CLI Emulator Guide

The `cor24-dbg` debugger lets you load, run, step through, and inspect
COR24 programs assembled with the reference `as24` toolchain.

## Quick Start

### 1. Assemble a program

```bash
# Assemble and link into LGO format
cor24-as < tests/programs/hello_uart.s | longlgo > hello_uart.lgo

# View the listing to see addresses and opcodes
cor24-as -l < tests/programs/hello_uart.s
```

### 2. Run in the debugger

```bash
cor24-dbg hello_uart.lgo
```

If the program's entry point is not at address 0 (e.g. sieve.lgo has
`_main` at 0x93):

```bash
cor24-dbg --entry 0x93 docs/research/asld24/sieve.lgo
```

### 3. Debug

```
(cor24) run                  # run until halt/breakpoint/limit
(cor24) step                 # single step one instruction
(cor24) step 10              # step 10 instructions
(cor24) info                 # show all registers
(cor24) disas                # disassemble at PC
(cor24) x /16 0xFF0100       # examine 16 bytes at UART address
```

## Command Reference

| Command | Short | Description |
|---------|-------|-------------|
| `run [N]` | `r` | Run N instructions (default 100M) |
| `step [N]` | `s` | Single step N instructions |
| `next` | `n` | Step over (skip into jal calls) |
| `continue` | `c` | Continue from breakpoint |
| `break <addr>` | `b` | Set breakpoint at address |
| `delete <N\|all>` | `d` | Delete breakpoint(s) |
| `info [r\|b]` | `i` | Show registers or breakpoints |
| `examine [/N] <addr>` | `x` | Examine N bytes at address |
| `print <reg\|addr>` | `p` | Print register or memory value |
| `disas [addr] [N]` | | Disassemble N instructions |
| `uart` | | Show UART output buffer |
| `led` | | Show LED D2 and button S2 state |
| `button [press\|release\|toggle]` | `btn` | Control button S2 |
| `load <file.lgo>` | | Load a new LGO file |
| `reset` | | Reset CPU to initial state |
| `quit` | `q` | Exit debugger |

Addresses accept decimal, `0x` hex, or `0b` binary.
An empty line repeats the last command (useful for repeated stepping).

## Use Cases

### Debugging a simple program step by step

```
$ cor24-dbg tests/programs/count_down.lgo
Loaded 22 bytes from tests/programs/count_down.lgo
PC = 0x000000

(cor24) disas 0 12
=> 000000: 80           push fp
   000001: 65           mov  fp,sp
   000002: 2B 00 01 FF  la   r2,0xFF0100
   000006: 44 05        lc   r0,5
   000008: 5A           mov  r1,r0
   000009: 0A 30        add  r1,48
   00000B: 85 00        sb   r1,(r2)
   00000D: 09 FF        add  r0,-1
   00000F: C8           ceq  r0,z
   000010: 14 F4        brf  0x000008
   000012: 69           mov  sp,fp
   000013: 7C           pop  fp

(cor24) b 0x0B
Breakpoint 1 at 0x00000B

(cor24) run
Breakpoint at 0x00000B
0x00000B: sb   r1,(r2)
  r0=000005 r1=000035 r2=FF0100 fp=FEEBFD sp=FEEBFD c=0

(cor24) p r1
r1 = 0x000035 (53)          # ASCII '5'

(cor24) c
5Breakpoint at 0x00000B      # printed '5', hit breakpoint again
0x00000B: sb   r1,(r2)
  r0=000004 r1=000034 r2=FF0100 fp=FEEBFD sp=FEEBFD c=0

(cor24) delete all
All breakpoints deleted.

(cor24) run
4321
Stopped after 100000000 instructions (limit).
```

### Inspecting LED and button state

```
$ cor24-dbg tests/programs/led_blink.lgo

(cor24) led
LED D2: OFF (bit0 = 0)
Button S2: HIGH (bit0 = 1)

(cor24) step 20
  r0=... r1=... ...

(cor24) led
LED D2: ON (bit0 = 1)

(cor24) button press
Button S2: pressed (LOW)

(cor24) step 5

(cor24) p 0xFF0000
[0xFF0000] = 0x00           # button reads low

(cor24) button release
Button S2: released (HIGH)
```

### Running the sieve benchmark

```
$ cor24-dbg --entry 0x93 docs/research/asld24/sieve.lgo
Loaded 346 bytes

(cor24) run 500000000
1000 iterations
1899 primes.

Stopped after 500000000 instructions (limit).

(cor24) uart
UART output buffer (29 chars):
1000 iterations
1899 primes.
```

### Examining UART I/O registers

```
(cor24) x /4 0xFF0100
0xFF0100: 00 02 00 00  |....|
           ^  ^
           |  +-- status: bit1=CTS active, bit7=TX not busy
           +---- data register
```

### Loading assembled programs

The full pipeline from source to debug session:

```bash
# 1. Write your program
cat > mytest.s << 'EOF'
_main:
    lc   r0,42
    la   r1,-65536       ; 0xFF0000 LED address
    sb   r0,0(r1)
_halt:
    bra  _halt
EOF

# 2. Assemble
cor24-as < mytest.s | longlgo > mytest.lgo

# 3. Debug
cor24-dbg mytest.lgo
```

## Memory Map

| Region | Address Range | Description |
|--------|---------------|-------------|
| SRAM | 0x000000 - 0x0FFFFF | 1MB main memory |
| EBR | 0xFEE000 - 0xFEFFFF | 8KB on-chip block RAM |
| Stack | grows down from 0xFEEC00 | Initial SP |
| LED/Switch | 0xFF0000 | bit 0: LED D2 / button S2 |
| Int Enable | 0xFF0010 | bit 0: UART RX interrupt |
| UART Data | 0xFF0100 | Read: RX data, Write: TX data |
| UART Status | 0xFF0101 | bit0=RX ready, bit1=CTS, bit7=TX busy |

## Tips

- Use `step` then press Enter repeatedly to keep stepping (empty line = repeat)
- Use `next` to step over function calls (`jal`)
- Branch targets in disassembly show the resolved address, not the raw offset
- The `run` command accepts a cycle count: `run 1000000` for 1M instructions
- Use `_` separators for readability: `run 500_000_000`
