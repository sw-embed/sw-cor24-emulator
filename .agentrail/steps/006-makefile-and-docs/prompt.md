Step 006 — migrate Makefile + docs to invoke cor24-asm.

Outstanding doc/script callsites of --run / --assemble (per the baseline grep, 62 hits across 11 files). After this step the only remaining user of these flags should be cli/src/run.rs itself (the flag handlers — those go in step 007).

What to migrate:
1. README.md - 4 invocations of cor24-run --run / --assemble.
2. docs/cli-tools.md - many cor24-run --run examples; the canonical replacement is cor24-asm <input>.s | cor24-emu --lgo - (or two-step via -o file). Update the Options table to drop --run / --assemble and add --lgo.
3. docs/eli5.md - tutorial examples.
4. docs/differentiate.md - feature differentiation list referencing the removed flags.
5. docs/feature-terminal-echo.md, docs/feature-terminal-mode.md, docs/feature-watchdog.md - feature docs with example invocations.
6. docs/migration-plan.md - if it documents --run / --assemble migration as a planned change, update or note done.
7. docs/vhs/asm-uart-hello.tape - VHS recording (may need re-recording — out of scope; leave a TODO).

Note: some docs reference an old binary name "cor24-run" (the renamed-ago predecessor of cor24-emu). If you find those, normalize to cor24-emu, but only as part of the same touch.

Important: examples/i2c/tmp101/Makefile uses --assemble. That file may not exist on this branch (it lands with the i2c work via feat/i2c-spi-emu). Check and:
- If it exists, migrate (replace --assemble with cor24-asm).
- If it does not exist, document in the commit message that the Makefile change must merge with the i2c branch arrival; do not block this saga.

CHANGES.md: leave historical mentions in place (they describe past releases).

Tests: docs are not tested by cargo. Run cargo build to confirm nothing broke.

Done when:
- git grep -nE -- "--run|--assemble" -- "docs/" "README.md" returns mostly historical CHANGES.md mentions + cli/src/run.rs flag handlers (still present, removed in step 007).
- The two test scripts (scripts/demo-cli-*.sh) already use cor24-dbg + .lgo, no migration needed.

Next step: --next-slug cli-flag-removal — delete --run / --assemble argv handling, help text, and command-not-found error path. Add a smoke test asserting the flags are gone.