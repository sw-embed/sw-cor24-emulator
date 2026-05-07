Step 002 — delete src/challenge.rs and migrate callers.

challenge.rs is dead code: it was wired to the deprecated Yew UI (already gone) and exists today only to seed examples in test code. Per mike (decision captured in step 001 baseline), delete it entirely.

What to remove:
1. src/challenge.rs — delete the file.
2. src/lib.rs — remove pub mod challenge and pub use challenge::{Challenge, get_challenges, get_examples}.

What to migrate (each site uses the assembler library; the assembler stays in this saga step but its surface gets thinner):
3. tests/integration_tests.rs — six tests use get_examples(): test_echo_example, test_count_down, test_fibonacci_example, test_hello_uart, test_multiply_example, test_interrupt_example. Each picks a named example from the list, assembles it, runs it. Replace with:
   - inline the source string at the test site (since the source is short and the test asserts specific UART/state outcomes), OR
   - move source to tests/programs/<name>.s (committed) and assemble at test build time via cor24-asm subprocess. Pick the simpler path — for short snippets, inline is fine.
4. src/emulator.rs::test_uart_log_echo_session — also uses get_examples(). Inline the relevant Echo source there too.
5. tests/integration_tests.rs use cor24_emulator::challenge::get_examples — drop the import.

Important: this step does NOT delete src/assembler.rs yet. The 6 #[cfg(test)] sites in src/emulator.rs that still use Assembler::new() (separate from challenge) keep working — they are step 004 work. Just the challenge import goes away.

Tests:
- cargo test --workspace passes — same test names should still exist after migration.
- Run cargo clippy --workspace -- -D warnings to confirm no dead-code/unused-import regressions from the dropped re-exports.

Done when:
- src/challenge.rs absent.
- src/lib.rs has no challenge module reference.
- tests/integration_tests.rs and src/emulator.rs no longer import challenge.
- All previously-passing tests still pass.

Next step: --next-slug demo-mode-as-lgo — pre-assemble DEMO_SOURCE via cor24-asm into a committed .lgo, replace the runtime Assembler path in --demo with include_str! + load_lgo.