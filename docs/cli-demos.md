# CLI demos

Each script in `scripts/demo-cli-*.sh` builds the CLI binaries and runs a
small program end-to-end. They double as sanity checks for the emulator
and as runnable references for new users.

All demos assume you are at the repo root. Each script handles its own
`cargo build`; you do not need to build first.

| Script | Program | What it shows |
|--------|---------|---------------|
| `demo-cli-hello-world.sh` | `tests/programs/hello_world.lgo` | UART string output; extracting UART text into a shell variable |
| `demo-cli-count-down.sh` | `tests/programs/count_down.lgo` | Breakpoints, single-stepping, register inspection in `cor24-dbg` |
| `demo-cli-led-blink.sh` | `tests/programs/led_blink.lgo` | LED toggling alongside UART output from a blink loop |
| `demo-cli-blinky-s2.sh` | `tests/programs/blinky_s2.lgo` | D2 LED mirrors S2 button via the shared `IO_LEDSWDAT` bit |
| `demo-cli-sieve.sh` | `docs/research/asld24/sieve.lgo` | Sieve of Eratosthenes benchmark (~500M instructions) |

## hello-world

```bash
scripts/demo-cli-hello-world.sh
```

Drives `cor24-dbg` against `hello_world.lgo`, captures the full debugger
session into `RAW_OUTPUT`, then uses `awk` to extract just the UART text
into `UART_OUTPUT`. Demonstrates the recommended pattern for using the
emulator in a shell pipeline (the same `awk` recipe is reproduced in
`docs/cli-emulator-guide.md`).

## count-down

```bash
scripts/demo-cli-count-down.sh
```

Runs `count_down.lgo` (prints `54321` to UART) through `cor24-dbg`:
disassemble, set a breakpoint at `0x0B`, run, inspect `r1`, continue,
clear breakpoints, run to completion, dump UART.

## led-blink

```bash
scripts/demo-cli-led-blink.sh
```

Runs `led_blink.lgo` — toggles LED D2 five times and prints `L` each
toggle. The script single-steps, samples the LED state with `led` between
chunks, and finally dumps UART. Useful for seeing the timing relationship
between an LED write and a UART transmit.

## blinky-s2

```bash
scripts/demo-cli-blinky-s2.sh
```

Runs `blinky_s2.lgo` (the 3-line MakerLisp program, see
[`docs/makerlisp-blinky_s2.md`](makerlisp-blinky_s2.md)) twice:

- once with S2 released (default) — expect `LED D2: 0x01 off`;
- once with `--switch on` — expect `LED D2: 0x00 ON (active-low)`.

Because `cor24-emu --switch` takes a single value at start-up, an actual
*toggle-during-run* test lives in the integration suite. The demo prints
the command to invoke it:

```bash
cargo test --test integration_tests test_blinky_s2_led_follows_switch
```

## sieve

```bash
scripts/demo-cli-sieve.sh
```

Runs `sieve.lgo` (entry `0x93`) for up to 500M instructions through
`cor24-dbg`, then prints the resulting UART output ("1000 iterations…"
and the prime counts). Acts as a coarse performance smoke test — if a
change makes this dramatically slower, that's a regression signal.

## Adding a new demo

1. Drop a `.lgo` (or `.s`) into `tests/programs/`.
2. Add a `scripts/demo-cli-<name>.sh` modeled on the others — build via
   `cargo build -p cor24-cli`, drive `cor24-emu` or `cor24-dbg` with a
   heredoc, and print clearly delimited sections.
3. `chmod +x` the script.
4. Append a row to the table at the top of this file.
5. If the demo exposes a behavior that benefits from automated checking,
   add a matching test in `tests/integration_tests.rs` (the blinky-s2
   demo follows this pattern).
