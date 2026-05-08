Step 005 — migrate tests/integration_tests.rs Assembler users.

Same migration pattern as step 004 but for tests/integration_tests.rs. The file currently:
1. use cor24_emulator::assembler::Assembler at line 3
2. const EXAMPLES (16 entries) - the include_str! list from step 002
3. test_all_examples_assemble - iterates EXAMPLES, calls Assembler::new() for each
4. test_all_examples_halt - same
5. test_fibonacci_example, test_multiply_example, test_uart_hello_example - call Assembler::new() against include_str source
6. test_self_branch_halt_via_step, test_step_halted_cpu_is_noop, test_memory_access_non_adjacent - go through assemble_and_run() helper that calls Assembler::new()
7. test_oom_example, test_stack_overflow_example - same via assemble_and_run
8. test_interrupt_example, test_echo_example - call Assembler::new() against include_str
9. test_uart_no_poll_drops_characters, test_uart_with_poll_all_characters, test_uart_never_ready_hangs_polling_program - inline source + Assembler::new()

What to do:
1. Add the same asm_to_lgo() helper as step 004 at the top of tests/integration_tests.rs (or factor it into a shared place if you want — but a duplicate small helper is fine and avoids exposing an internal crate API).
2. Migrate assemble_and_run() helper to use asm_to_lgo + load_lgo.
3. Migrate the per-test direct calls to Assembler::new().
4. test_all_examples_assemble: now becomes a "all examples can be assembled by cor24-asm" smoke test - iterate EXAMPLES, call asm_to_lgo for each, assert the result is non-empty (or assert it parses as valid lgo).
5. Drop "use cor24_emulator::assembler::Assembler" from tests/integration_tests.rs.

Done when:
- tests/integration_tests.rs has no references to cor24_emulator::assembler.
- All 21 integration tests still pass.

Next step: --next-slug makefile-and-docs — migrate examples/i2c/tmp101/Makefile (will land with i2c later; may not exist on this branch — if so, document for the merge), then update README.md, docs/cli-tools.md, docs/eli5.md, docs/differentiate.md, docs/feature-*.md examples to invoke cor24-asm + cor24-emu --lgo / --load-binary.