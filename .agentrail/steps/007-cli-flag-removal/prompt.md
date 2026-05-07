Step 007 — remove --run and --assemble from cli/src/run.rs.

cli/src/run.rs is the last user of crate::assembler. After this step, src/assembler.rs has no callers and step 008 deletes it.

What to remove from cli/src/run.rs:
1. The struct CliArgs fields that exist only for these flags: file (used by --run/--assemble), base_addr (used only by --assemble for label resolution).
   Actually file is also used by binary-mode parsing. Keep file. Drop base_addr (its only consumer is --assemble per the baseline).
2. parse_args match arms for "--run" and "--assemble".
3. The "run" and "assemble" branches in main()s match cli.command.as_str().
4. The "Unknown command" error message lists --run and --assemble — update to suggest --lgo and --demo.
5. Help text in print_short_help: 4 example invocations + the --run flag entry + the --assemble flag entry. Replace examples with --lgo equivalents (the cor24-emu --demo example stays). Drop the entries for the removed flags.
6. print_long_help references — update similarly.
7. The "use cor24_emulator::assembler::{Assembler, AssemblyResult}" import in cli/src/run.rs (line 9 in the baseline) — drop.
8. load_assembled() helper — was used by --run/--assemble paths. Likely unused after removal; drop if so.

Important:
- Keep --demo (was step 003 — uses include_str of the .lgo, no Assembler).
- Keep --load-binary, --patch, all observability/control flags.
- Add a smoke test in cli/src/run.rs cfg(test): assert that argv with --run or --assemble exits non-zero with a clear message ("Unknown command"); assert --help shows neither flag.
- Note: --base-addr only consumer is --assemble per baseline. Drop CliArgs::base_addr and its parse arm. If grep finds another user, leave it.

Tests: cargo build, cargo test --workspace, cargo clippy. cor24-emu --help should no longer mention --run or --assemble.

Done when:
- cli/src/run.rs has no references to crate::assembler / cor24_emulator::assembler.
- cor24-emu --help shows neither --run nor --assemble.
- A smoke test confirms invoking those flags exits non-zero.
- All previously-passing tests still pass.

Next step: --next-slug delete-assembler-module — rm src/assembler.rs + lib.rs pub use. Final: --next-slug final-sweep for grep verification, clippy, and end-of-saga sign-off.