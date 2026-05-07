Step 009 — final sweep + saga sign-off.

What to verify:
1. git grep -nE "use cor24_emulator::assembler|crate::assembler|::assembler::" excluding the saga doc returns zero hits.
2. git grep -nE -- "--run|--assemble" excluding intentional historical record (CHANGES.md, migration-plan.md, saga baseline doc) returns: only the parser-arm rejection in cli/src/run.rs (intentional) plus comments.
3. cor24-emu --help shows neither --run nor --assemble; advertises --lgo.
4. cor24-emu --run prog.s exits non-zero with the migration message; same for --assemble.
5. cargo test --workspace passes — record final test count.
6. cargo clippy --workspace --all-targets -- -D warnings is clean.
7. No wasm-bindgen / web-sys / js-sys imports anywhere (grep src/ cli/ tests/ examples/).
8. Update docs/saga-remove-internal-assembler-baseline.md with a closing section: final test count, final flag inventory, deltas vs baseline.

Wrap-up:
- Run agentrail complete --done after the final sweep verification.
- Signal mike for the relay (per brief: push pr/remove-internal-assembler and notify; mike runs dg-relay).