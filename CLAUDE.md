# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workflow modes

This repo predates AgentRail. It operates in one of two modes:

- **Maintenance mode (default, current state):** No `.agentrail/` saga exists. Run small fixes, doc updates, and ad-hoc work directly. No `agentrail` calls are required or expected.
- **Saga mode:** A planned feature with non-trivial scope (e.g. an upcoming `feature-*.md` plan in `docs/`) gets its own saga via `agentrail init --name <slug> --plan docs/feature-<slug>.md`. Use `init` (not `setup`) so this hand-curated CLAUDE.md is left alone.

### AgentRail Session Protocol — applies only when a saga is active

When `.agentrail/` exists, every session follows this exact sequence:

1. `agentrail next` — read your current step, prompt, skill docs, and past trajectories.
2. `agentrail begin` — mark the step in-progress.
3. Do what the step prompt says. The prompt IS your instruction — do not ask "shall I start?"
4. Commit your code changes with git.
5. `agentrail complete --summary "..." --reward 1 --actions "..." --next-slug "..." --next-prompt "..." --next-task-type "..."` (use `--reward -1 --failure-mode "..."` if the step failed; add `--done` if the saga is finished).
6. STOP. No further code changes after `complete` — they are untracked and invisible to the next session.

In maintenance mode, skip steps 1, 2, 5, 6 entirely.

## Branching policy (devgroup workflow)

This repo lives under the devgroup workflow. The full policy is at `/disk1/github/softwarewrighter/devgroup/docs/branching-pr-strategy.md`; a summary is printed by the `onboarding` script in `$PATH`.

- `main` and `dev` are coordinator-only. **Never push.**
- Base new work on `origin/dev` (or `origin/main` if `dev` doesn't exist yet — onboarding flags that case).
- Use `feat/<slug>` (or `fix/<slug>`) for in-progress work; rename to `pr/<slug>` via `dg-mark-pr` when ready to merge. The coordinator relays `pr/*` into `dev`.
- Helpers in `$PATH`: `dg-new-feature`, `dg-new-fix`, `dg-mark-pr`, `dg-list-pr`, `dg-reap`.

## Multi-Agent Coordination (Wiki)

This project coordinates with other agents via a shared wiki. See `docs/agent-cas-wiki.md` for the API reference and CAS protocol.

- **Wiki server:** `http://localhost:7402` (git backend). May not be running in every environment — check before relying on it.
- **Key pages:** [[AgentToAgentRequests]], [[AgentStatus]], [[COR24RS]], [[COR24Toolchain]], [[MVP]].
- **Our role:** the foundation layer — every COR24 project depends on our assembler and emulator.
- **On session start (when wiki is reachable):** read [[AgentToAgentRequests]] for requests targeting this repo; update [[AgentStatus]].

## Related Projects

- `~/github/sw-vibe-coding/pv24a` — P-code VM and p-code assembler (COR24 assembly)
- `~/github/softwarewrighter/pa24r` — P-code assembler (Rust, .spc → .p24)
- `~/github/softwarewrighter/pl24r` — P-code text-level linker (Rust)
- `~/github/softwarewrighter/p24p` — Pascal compiler (C, compiled by tc24r)
- `~/github/softwarewrighter/pr24p` — Pascal runtime library (.spc sources)
- `~/github/softwarewrighter/web-dv24r` — Browser-based p-code VM debugger
- `~/github/sw-vibe-coding/tc24r` — COR24 C compiler (Rust)
- `~/github/sw-vibe-coding/agentrail-domain-coding` — Coding skills domain

## Build & Test

```bash
# Build and test the whole workspace (lib + cli + isa)
scripts/build.sh                # cargo build --workspace && cargo test --workspace

# Or invoke cargo directly
cargo build --workspace
cargo test --workspace
cargo check
cargo clippy --workspace
```

The CLI binaries land in `target/debug/` (or `target/release/`):

- `cor24-emu` — assembler + emulator runner. `--demo`, `--run <file.s>`, `--assemble in.s out.bin out.lst`, `--terminal`, `--dump-uart`, `--uart-file <path>`, `--speed`, `--time`. See `docs/cli-emulator-guide.md`.
- `cor24-dbg` — GDB-like CLI debugger. Loads `.lgo` files, `--entry <addr>`. See `docs/cli-tools.md`.

Demo runners are in `scripts/demo-cli-*.sh` (hello-world, count-down, led-blink, sieve).

## Commit Discipline

**Commit early and often.** Each commit should do one thing. Do not accumulate large changesets.

- Commit after each logical change: a bug fix, a new feature, a refactor, an extraction — each is its own commit.
- Small commits enable cherry-picking, rebasing, and bisecting. Large commits make all of these painful.
- If a task involves multiple steps (e.g., extract data, then update callers, then add a new feature), commit after each step.
- Commit working code. Run `cargo test --workspace` before committing.

## Architecture

This is a Rust emulator for the COR24 CPU — a real 24-bit RISC architecture (C-Oriented RISC) designed for embedded systems education. The workspace produces a library and two CLI tools.

### Workspace layout (`Cargo.toml` members: `.`, `cli`, `isa`)

- **Root crate `cor24-emulator` (`src/`)** — emulator library.
  - `cpu/` — CPU state, memory (24-bit address space subset), memory-mapped I/O, decode ROM (extracted from FPGA Verilog), executor, instruction definitions, encoding tables.
  - `assembler.rs` — two-pass assembler producing machine code from COR24 assembly. Supports `#x` and `0x` hex prefixes.
  - `emulator.rs` — `EmulatorCore` higher-level wrapper used by both CLIs.
  - `loader.rs` — loads `.lgo` and other binary formats into emulator memory.
  - `challenge.rs` — example programs and challenge definitions.
  - `examples/` — sample programs.
- **`cli/` crate `cor24-cli`** — two binaries (`cor24-emu`, `cor24-dbg`) thin-wrapping the library; depends on `cor24-emulator`. Both crates have a `build.rs` that shells out to `git`/`date`/`hostname` and emits `VERGEN_GIT_SHA_SHORT`, `VERGEN_BUILD_TIMESTAMP`, `VERGEN_BUILD_HOST` (`vergen` itself is not a dependency — the names are kept for compatibility).
- **`isa/` crate `cor24-isa`** — opcode definitions, encoding tables, branch constants. Optional `serde` feature. Used by the root crate.

### Memory-mapped I/O

Hardware-accurate to the COR24-TB test board:

- `0xFF0000` — `IO_LEDSWDAT`: bit 0 is LED D2 (active-low) and switch S2.
- `0xFFFF00`–`0xFFFF02` — UART (status + data). The CLI captures a chronological UART transaction log (`--dump-uart`, `--uart-file`).

Recent CLI work has added: piped-stdin buffering for `--terminal` mode, stack overflow/underflow detection (`set_stack_bounds`), and control-flow guards. See `CHANGES.md`.

### Pre-built web UI in `pages/`

`pages/` contains pre-built WASM/JS artifacts from a prior Yew-based web UI incarnation, deployed by `.github/workflows/deploy.yml` on push to `main`. The Yew/Trunk source tree was removed in commit `a248c7b refactor: trim repository to emulator-only scope`; the artifacts were re-added in `f762bb7` to keep the GitHub Pages site alive but are effectively frozen — there is no longer any way to rebuild them from this repo. Do not assume `./serve.sh`, `./build.sh`, `Trunk.toml`, `index.html`, or `trunk` exist; they don't. If the web UI needs real changes, that work has to be re-scoped from scratch (likely as a separate repo per the refactor commit message).

### Tests

`cargo test --workspace` runs unit tests in each crate plus `tests/integration_tests.rs` (integration tests against `tests/programs/`).

### Reference Materials

- `docs/isa-reference.md`, `docs/assembler-examples.md`, `docs/cli-tools.md`, `docs/cli-emulator-guide.md` — primary docs.
- `docs/feature-*.md` — per-feature planning docs (the model for future feature plans, including upcoming I2C/SPI memio work).
- `CHANGES.md` — chronological changelog.
- `docs/fpga-soft/` and historical references — hardware context.
