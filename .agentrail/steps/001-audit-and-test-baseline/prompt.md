Step 001 — pre-removal baseline.

Capture the snapshot we will compare against at end-of-saga. No code changes yet.

What to do:
1. Run cargo test --workspace and record the per-target test counts (lib, integration_tests, web_surface if present, cli, isa).
2. Run cor24-emu --help and capture the flag list (or assert which flags are present that this saga will remove: --run, --assemble; and which it must keep: --demo, --lgo, --load-binary, --patch, --entry, --uart-input, --uart-file, --terminal, --echo, --quiet, --dump, --dump-uart, --speed, --time, --max-instructions, --guard-jumps, --canary, --watch-range, --switch, --uart-never-ready, --stack-kilobytes, --code-end, --base-addr).
3. Run git grep -nE -- "--run|--assemble" -- cli scripts tests docs README.md CHANGES.md examples and capture the hit count + file list.
4. Verify cor24-asm is on PATH and produces all three outputs end-to-end against a 2-line .s smoke fixture.
5. Note any callsites the brief did NOT enumerate but that we discovered (so far: src/challenge.rs deletion is in scope per mike, src/emulator.rs has 6 #[cfg(test)] Assembler users, --demo embeds DEMO_SOURCE assembled at runtime).

Commit a short docs/feature-remove-internal-assembler.md (or similar) summarizing the baseline. No production code touched in this step.

Done when:
- cargo test --workspace runs green and you have a recorded baseline number.
- The grep result is captured (count + file list).
- A baseline doc is committed at docs/saga-remove-internal-assembler-baseline.md (or similar — match the local naming convention).

Next step:
--next-slug delete-challenge-rs — rip src/challenge.rs + lib.rs re-exports + 7 known callsites; migrate via inline source + cor24-asm subprocess or new tests/programs/*.s fixtures.