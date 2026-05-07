# Baseline: pr/remove-internal-assembler

Captured at the start of saga step 001. Compare against this when the
saga lands.

**Branch base:** `pr/remove-internal-assembler` cut from `origin/dev`
(`4841e9d`). The I2C work on `feat/i2c-spi-emu` is not yet on `dev`.

**Brief:** `/disk1/github/softwarewrighter/devgroup/tools/briefs/dcemu-remove-internal-assembler.md`.

**`cor24-asm`:** `/disk1/github/softwarewrighter/devgroup/work/bin/cor24-asm`
v0.1.0. Smoke-tested end-to-end — `.lgo`, `.bin`, `.lst` all written
from a 4-line input.

## Test counts (`cargo test --workspace`)

| Target                       | Pass |
|------------------------------|-----:|
| cor24-emulator lib           | 207  |
| tests/integration_tests.rs   |  21  |
| cor24-cli unit tests         |  12  |
| cor24-isa lib                |   3  |
| **Total**                    | 243  |

All targets pass; zero ignored, zero failed. End of saga must still
pass with no regressions to non-removed tests.

## Help text — flag inventory

`cor24-emu --help` Usage block:

```
cor24-emu --demo [options]                                      keep
cor24-emu --run <file.s> [opts]                                 REMOVE
cor24-emu --load-binary <f>@<a> --entry <a>                     keep
cor24-emu --assemble <in.s> <out.bin> <out.lst>                 REMOVE
```

Options to **keep** (verified present): `-h`, `--help`, `-V`,
`--speed/-s`, `--time/-t`, `--max-instructions/-n`, `--uart-input/-u`,
`--uart-file`, `--quiet/-q`, `--entry/-e`, `--dump`, `--dump-uart`,
`--trace`, `--step`, `--terminal`, `--echo`, `--load-binary`, `--patch`,
`--base-addr`, `--stack-kilobytes`, `--switch`, `--uart-never-ready`,
`--guard-jumps`, `--code-end`, `--canary`, `--watch-range`.

Options that don't exist on this branch yet (will land with the I2C
merge from `feat/i2c-spi-emu`): `--lgo`, `--i2c-device`, `--dump-i2c`.
The brief lists `--lgo` as the replacement for `--run` — when this
saga merges with the I2C work, that flag will already exist. For this
saga's scope, the closest analogue today is `--load-binary` for the
runtime path; the public `EmulatorCore::load_lgo` API exists in
`src/loader.rs` and `src/emulator.rs:98`.

`--base-addr` is currently documented as "Base address for assembly
(default: 0)" — its only consumer is `--assemble` mode. With
`--assemble` removed, `--base-addr` becomes orphaned and is removed
along with it (saga step 007).

## Internal assembler usage

`grep -nE -- '--run|--assemble'` across `cli/`, `scripts/`, `tests/`,
`docs/`, `README.md`, `CHANGES.md`, `examples/`: **62 hits across
11 files**.

| File                                | Notes                                        |
|-------------------------------------|----------------------------------------------|
| `CHANGES.md`                        | Historical mentions — keep as record         |
| `README.md`                         | 4 example invocations to migrate             |
| `cli/src/run.rs`                    | The flag handlers themselves + 9 help-text examples |
| `docs/cli-tools.md`                 | Many `cor24-run --run` examples to migrate   |
| `docs/differentiate.md`             | Feature differentiation list                 |
| `docs/eli5.md`                      | Tutorial usage examples                      |
| `docs/feature-terminal-echo.md`     | Feature doc example invocation               |
| `docs/feature-terminal-mode.md`     | Feature doc example invocation               |
| `docs/feature-watchdog.md`          | Feature doc example invocation               |
| `docs/migration-plan.md`            | Migration-related doc                        |
| `docs/vhs/asm-uart-hello.tape`      | VHS demo recording script                    |

Library code that imports the assembler (will need migration or
deletion):

| File                          | Reason                                                      |
|-------------------------------|-------------------------------------------------------------|
| `src/lib.rs:21`               | `pub use assembler::{...}` — remove with module             |
| `cli/src/run.rs:9`            | `use cor24_emulator::assembler::{Assembler, AssemblyResult}` — `--run`/`--assemble`/`--demo` use it |
| `src/challenge.rs:3`          | `use crate::assembler::Assembler` — DELETE the whole module per mike |
| `src/emulator.rs` (6 sites)   | All `#[cfg(test)]` — migrate to pre-built fixtures or `cor24-asm` subprocess |
| `tests/integration_tests.rs:3,4` | Imports `assembler::Assembler` and `challenge::get_examples` — migrate consumers |

Library exports to remove (`src/lib.rs:21-22`):
`AssembledLine`, `Assembler`, `AssemblyResult`, `Challenge`,
`get_challenges`, `get_examples`.

## Decisions

- `src/challenge.rs` — delete entirely.
- `--demo` mode — pre-assemble `DEMO_SOURCE` via `cor24-asm` once,
  embed the resulting `.lgo` text via `include_str!`, replace the
  runtime `Assembler::new()` path with `EmulatorCore::load_lgo`.

## Done when (this saga)

- `cargo test --workspace` ≥ 243 tests pass with no regressions
  (test count may decrease as tests-of-the-removed-assembler are
  deleted along with their target).
- `git grep -nE -- '--run|--assemble'` returns only `CHANGES.md`
  historical mentions.
- `cor24-emu --help` shows neither `--run` nor `--assemble`.
- `src/assembler.rs` and `src/challenge.rs` are gone; their re-exports
  are gone; their callers are migrated.
- `cargo clippy -- -D warnings` clean.
- No `wasm_bindgen`/`web_sys`/`js_sys` imports anywhere.

## Closing section (after step 009)

| Gate                                          | Result                                                                      |
|-----------------------------------------------|-----------------------------------------------------------------------------|
| `cargo test --workspace`                      | 203 pass, 0 fail (lib 166 + integration 21 + cli unit 11 + cli integration 2 + isa 3) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean                                                              |
| `cor24-emu --help`                            | advertises `--lgo`, `--demo`, `--load-binary`; no `--run` / `--assemble`     |
| `cor24-emu --run prog.s` / `--assemble`       | exits 2 with migration message pointing at `cor24-asm`                      |
| Active references to `cor24_emulator::assembler` | zero (only this baseline doc retains historical mentions)                |
| Active references to `crate::assembler`       | zero                                                                        |
| `wasm_bindgen` / `web_sys` / `js_sys` imports | zero                                                                        |
| `src/assembler.rs`                            | deleted (1512 lines)                                                        |
| `src/challenge.rs`                            | deleted (514 lines)                                                         |
| `cli/tests/removed_flags.rs`                  | new — pins the `--run`/`--assemble` rejection contract                      |
| `examples/i2c/tmp101/Makefile`                | not present on this branch (lands with `feat/i2c-spi-emu`); when that branch merges to `dev`, its `--assemble` invocation must also be migrated |

### Test-count delta

| Target                       | Baseline | Final | Delta |
|------------------------------|---------:|------:|------:|
| `cor24-emulator` lib         |      207 |   166 |   −41 |
| `tests/integration_tests.rs` |       21 |    21 |     0 |
| `cor24-cli` unit             |       12 |    11 |    −1 |
| `cor24-cli` integration      |        0 |     2 |    +2 |
| `cor24-isa` lib              |        3 |     3 |     0 |
| **Total**                    |  **243** |**203**| **−40** |

The −41 in lib comes from the assembler module's own unit tests
(`assembler::tests::test_*`) which went away with the file. The −1
in cli unit is the deleted `test_assemble_at_base_and_load`. The
+2 in cli integration is the new `removed_flags.rs` smoke tests.
Net: every removed test was either testing the deleted module or
testing a deleted flag. Zero regressions in surviving tests.

### Commits in this saga

```
1f4252b refactor(cli): remove --run and --assemble; reject with migration message
b6a90cf docs: migrate --run / --assemble examples to cor24-asm + --lgo
5b4cf39 test(integration): migrate Assembler users to cor24-asm
426487f test(emulator): migrate cfg(test) Assembler users to cor24-asm
000e894 (older) ... (saga steps before this one)
7ddb3a5 refactor: delete src/assembler.rs and lib.rs re-exports
```

Ready for relay via `dg-relay dcemu sw-cor24-emulator pr/remove-internal-assembler`.
