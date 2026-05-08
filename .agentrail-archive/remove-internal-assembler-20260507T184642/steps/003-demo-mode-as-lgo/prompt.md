Step 003 — embed --demo as a pre-built .lgo.

Today cli/src/run.rs::main matches "demo" command by assembling DEMO_SOURCE (a multi-line literal in cli/src/run.rs) at runtime via Assembler::new(). Per the saga decision, replace this runtime-assembly path with a committed .lgo embedded via include_str! and loaded via EmulatorCore::load_lgo.

What to do:
1. Locate DEMO_SOURCE in cli/src/run.rs (search for "DEMO_SOURCE" — it is a const &str around line 100-200).
2. Write the source to a file: cli/src/demo.s (or similar — match local convention; could also live in src/examples/assembler/demo.s if that fits).
3. Run cor24-asm <demo.s> -o cli/src/demo.lgo to produce the canonical .lgo. Commit BOTH the .s and the .lgo.
4. Update the "demo" command branch in cli/src/run.rs:
   - Drop the Assembler::new() / asm.assemble(DEMO_SOURCE) / load_assembled() pattern.
   - Replace with let lgo = include_str!("demo.lgo"); emu.load_lgo(lgo, None).expect(...);
   - Drop the program-listing print loop that used result.lines, OR replace with a simple "Loaded N bytes from demo.lgo" line. Match how the lgo command branch handles this (it prints "Loaded N bytes from <filename>"; do similar but with include_str!s static path).
5. Verify: cargo build + cor24-emu --demo runs end-to-end (LED counter visible at high speeds).

Important: cor24-asm is the canonical tool now. Do NOT use the internal Assembler to generate demo.lgo — use the standalone cor24-asm binary at /disk1/github/softwarewrighter/devgroup/work/bin/cor24-asm. The two assemblers may differ in edge cases; cor24-asm is the production output going forward.

Tests:
- cargo test --workspace passes (no test names changed by this step).
- cor24-emu --demo --speed 0 -n 1000 runs without crashing (smoke).
- The demo.lgo content is regenerable: include a short `make` target or a comment in demo.s noting the cor24-asm command.

Done when:
- DEMO_SOURCE is no longer referenced in cli/src/run.rs.
- demo.lgo is committed alongside demo.s.
- cor24-emu --demo still works.
- Assembler::new() is no longer called from the demo command branch.

Next step: --next-slug emulator-tests-migration — 6 #[cfg(test)] sites in src/emulator.rs that build source via Assembler::new() get migrated to either pre-built .lgo via cor24-asm at fixture-build time, or kept inline source for now (decision in step 004).