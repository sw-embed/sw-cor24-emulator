# makerlisp blinky_s2 — D2 follows S2

A 27-byte COR24 program from the MakerLisp/COR24-TB world: hold down
button **S2** and LED **D2** lights up; release **S2** and **D2** goes
dark. The whole thing fits in three lines of `.lgo`.

## The program

`tests/programs/blinky_s2.lgo`:

```
L000000807F7E652B0000FF2E00820013F82900ECFE662A00E0FEC7000000
; Blinky - D2 LED follows S2 button press
G00000E
```

- `L000000…` — one load record: 27 bytes starting at address `0x000000`.
- `;` — comment.
- `G00000E` — start execution at `0x00000E`.

## Disassembly

```
; --- loop body (entered via call from startup) ---
000000: 80           push fp
000001: 7F           push r2
000002: 7E           push r1
000003: 65           mov  fp,sp
000004: 2B 00 00 FF  la   r2,0xFF0000   ; r2 = IO_LEDSWDAT
000008: 2E 00        lb   r0,(r2)       ; r0 = switch byte (bit 0 = S2)
00000A: 82 00        sb   r0,(r2)       ; write same byte -> drives LED D2
00000C: 13 F8        bra  0x000008      ; forever

; --- entry point (G00000E) ---
00000E: 29 00 EC FE  la   r0,0xFEEC00   ; top of EBR stack
000012: 66           mov  sp,r0         ; sp = 0xFEEC00
000013: 2A 00 E0 FE  la   r1,0xFEE000   ; r1 = bottom of EBR
000017: C7 00 00 00  call 0x000000      ; jump into the loop (never returns)
```

Reproduce with `cor24-dbg`:

```bash
./target/debug/cor24-dbg tests/programs/blinky_s2.lgo <<<'disas 0 30'
```

## Analysis

`IO_LEDSWDAT` at `0xFF0000` is the trick: bit 0 of the **same byte** is
**S2 on read** and **D2 on write** (both active-low — `1` = released /
off, `0` = pressed / on). The loop is therefore three instructions:

1. `lb r0,(r2)` — read S2 into bit 0 of `r0`.
2. `sb r0,(r2)` — write that bit back, so D2 mirrors S2.
3. `bra 0x000008` — spin.

Startup at `0x0E` sets `sp` to the top of EBR (`0xFEEC00`), parks `r1`
at the bottom of EBR (`0xFEE000` — bookkeeping the program never uses
again), and `call`s into the loop. The prologue at `0x00`
(`push fp / push r2 / push r1 / mov fp,sp`) burns three slots of stack
and is never unwound; that's fine because the loop never returns.

Anything past `0x1B` reads as `0x00` (`add r0,r0`) but is never executed.

## Testing

### 1. S2 released (default)

```bash
./target/debug/cor24-emu --lgo tests/programs/blinky_s2.lgo \
  --time 1 --speed 100000 --dump 2>&1 | tail -5
```

Expect `LED D2: 0x01  off`.

### 2. S2 pressed at boot

The `--switch on` flag pins S2 low for the whole run:

```bash
./target/debug/cor24-emu --lgo tests/programs/blinky_s2.lgo \
  --switch on --time 1 --speed 100000 --dump 2>&1 | tail -5
```

Expect `LED D2: 0x00  ON (active-low)` and `BTN S2: 0x00  PRESSED`.

### 3. Toggle S2 in a loop (integration test)

The CLI takes a single `--switch` value at start-up, so for a live
toggle we drive the emulator from Rust. The test in
`tests/integration_tests.rs` (`test_blinky_s2_led_follows_switch`)
does this directly:

```rust
let executor = Executor::new();
executor.run(&mut cpu, 50); // run startup + a few loop iterations

for cycle in 0..5 {
    cpu.io.switches = 0x00;       // press S2
    executor.run(&mut cpu, 30);
    assert_eq!(cpu.io.leds, 0x00, "cycle {cycle}: D2 should be on");

    cpu.io.switches = 0x01;       // release S2
    executor.run(&mut cpu, 30);
    assert_eq!(cpu.io.leds, 0x01, "cycle {cycle}: D2 should be off");
}
```

Run just this test:

```bash
cargo test --test integration_tests test_blinky_s2_led_follows_switch
```

Or the whole workspace:

```bash
cargo test --workspace
```
