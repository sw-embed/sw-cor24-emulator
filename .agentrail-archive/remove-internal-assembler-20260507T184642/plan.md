# Saga: remove the internal assembler from cor24-emu

**Brief:** `/disk1/github/softwarewrighter/devgroup/tools/briefs/dcemu-remove-internal-assembler.md`

## Goal

Turn `cor24-emu` into a pure runtime consumer. Delete `src/assembler.rs`,
remove `--run` and `--assemble`, migrate every callsite (demos, tests,
docs) to invoke `cor24-asm` first then `cor24-emu --lgo` (or
`--load-binary`).

## Architectural boundary (recap)

- `.lgo` reader/loader → here. Stays.
- Assembler / writers (`.lgo`, `.bin`, `.lst`) → `sw-cor24-x-assembler`.
- Runtime → here. Consumes `.lgo` (`--lgo`) and raw bytes
  (`--load-binary`). Does not know how artifacts were built.

## What to remove

1. `cor24-emu --run <file.s>` — argv, dispatch, help text.
2. `cor24-emu --assemble <in.s> <out.bin> <out.lst>` — same.
3. `src/assembler.rs` — the internal assembler library.
4. `src/lib.rs` re-exports of the assembler.
5. `cor24_emulator::assembler` imports in `cli/src/run.rs` and any
   internal callers used solely for `--run` / `--assemble`.
6. Help-text examples for the removed flags.
7. Tests that exist solely to test `cor24_emulator::assembler`.

## What to keep

- Everything else: `--demo`, `--lgo`, `--load-binary`, `--patch`,
  `--entry`, all UART / terminal / observability flags, all execution
  modifiers (`--speed`, `--time`, etc.).
- `cor24-dbg` binary — never used the internal assembler.
- `src/loader.rs` — the `.lgo` parser stays exactly as-is.

## Decisions captured for two open questions

- **`src/challenge.rs`**: delete entirely. Its `get_challenges()` /
  `get_examples()` are only consumed by the deprecated Yew UI (already
  gone) and by tests. Tests are migrated.
- **`--demo` mode**: pre-assemble the embedded `DEMO_SOURCE` via
  `cor24-asm` once, commit the result as a `.lgo` text artifact, swap
  the runtime `Assembler::new()` path in `cli/src/run.rs` for
  `load_lgo` against the embedded `.lgo`. Keeps `--demo` working with
  no internal assembler.

## Step plan

```
001  audit-and-test-baseline       Baseline cargo test count, --help, grep state.
002  delete-challenge-rs           Rip src/challenge.rs + lib.rs re-exports;
                                   migrate 7 callsites (6 in tests/integration_tests.rs,
                                   1 in src/emulator.rs::test_uart_log_echo_session).
                                   Use inline source + cor24-asm subprocess or new
                                   tests/programs/*.s fixtures.
003  demo-mode-as-lgo              cor24-asm DEMO_SOURCE → demo.lgo, commit; swap
                                   cli/src/run.rs's Assembler::new() in --demo path
                                   for include_str! + load_lgo.
004  emulator-tests-migration      6 #[cfg(test)] sites in src/emulator.rs that
                                   build source → cor24-asm at fixture-build time
                                   or via subprocess from the test.
005  integration-tests-migration   tests/integration_tests.rs assembler import
                                   removed; remaining callers (besides examples)
                                   migrated.
006  makefile-and-docs             examples/i2c/tmp101/Makefile (will land with i2c
                                   merge — coordinate); README.md, docs/cli-tools.md,
                                   docs/eli5.md, docs/differentiate.md, docs/feature-*.md.
007  cli-flag-removal              Remove --run / --assemble argv handling, help
                                   text, command-not-found error path. Add a smoke
                                   test that the flags are gone and exit non-zero.
008  delete-assembler-module       rm src/assembler.rs + lib.rs `pub use assembler::*`
                                   + cli/src/run.rs `use cor24_emulator::assembler::*`.
                                   cargo test --workspace still green.
009  final-sweep                   grep -nE -- '--run|--assemble' returns only
                                   historical CHANGES.md mentions; clippy clean;
                                   no wasm; cargo test --workspace green;
                                   --help shows no removed flags.
```

## When done

Push `pr/remove-internal-assembler` and notify mike.

## Coordination notes

- Cut from `dev`. The I2C work on `feat/i2c-spi-emu` is not yet on
  `dev`; when it lands, this branch's removal will need to merge with
  the I2C additions. No expected conflicts (I2C added flags this saga
  doesn't touch).
- `cor24-asm` is at `/disk1/github/softwarewrighter/devgroup/work/bin/cor24-asm`.
- This saga is removal-only. No new emulator features, no new format
  spec docs, no changes to `sw-cor24-x-assembler`.
