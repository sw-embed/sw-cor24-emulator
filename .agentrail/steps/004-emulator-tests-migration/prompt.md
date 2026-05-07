Step 004 — migrate src/emulator.rs cfg(test) Assembler users.

Six #[cfg(test)] sites in src/emulator.rs build source via Assembler::new() then run it. They are tests of the emulator runtime that incidentally use the assembler. With src/assembler.rs going away in step 008, these need migration. Find them with: grep -n "use crate::assembler::Assembler\|crate::assembler::Assembler::new" src/emulator.rs.

Decision rule: keep the test, change how it gets bytes into the emulator.

Two options per site:
A) Inline source unchanged + keep Assembler::new() for now. Step 008 will rip them when src/assembler.rs goes; if a test is purely an Assembler smoke test, mark it for deletion in step 008 prompt.
B) Pre-assemble the source via cor24-asm at the test invocation time (Command::new) into a temp file, then load_lgo. Heavier but matches the production toolchain.

Recommended approach: option C — keep tests behaviour-identical for now, but move source out of inline strings into per-test .s files under tests/programs/ if not there already, and have the test go through cor24-asm via std::process::Command. This pre-stages step 008 cleanly: when assembler.rs is deleted, these tests already work without it.

If the test is short and purely tests an Assembler edge case (not emulator behaviour), mark it for removal in the step 008 prompt rather than migrating.

What to do:
1. Inventory the 6 sites: grab their function names, what they test, and length.
2. For each:
   - If it tests EMULATOR behaviour using assembled output as input -> migrate to subprocess cor24-asm + load_lgo.
   - If it tests ASSEMBLER behaviour -> note for deletion in step 008.
3. Add a small helper at the top of the cfg(test) module:
     fn asm_to_lgo(source: &str) -> String { /* shells out to cor24-asm */ }
   so the migration is one helper call per site.
4. Run cargo test --workspace and confirm the same test names pass.

Tests: 243 pass before, must equal or exceed 243 after migration (no test deletions in this step).

Done when:
- src/emulator.rs has at most 0 references to crate::assembler (all migrated through cor24-asm subprocess) OR documented exceptions.
- cor24-asm needs to be on PATH when cargo test runs (mike has confirmed it is for our environment).
- All previously-passing tests still pass.

Next step: --next-slug integration-tests-migration — same pattern for tests/integration_tests.rs (the EXAMPLES table from step 002 + the inline-source tests).