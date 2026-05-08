Step 008 — delete src/assembler.rs and its re-exports.

Now that no callers remain (challenge.rs gone, --run/--assemble gone, all tests through cor24-asm subprocess), the in-tree assembler module is dead. Delete it:

1. rm src/assembler.rs.
2. src/lib.rs: remove pub mod assembler and pub use assembler::{AssembledLine, Assembler, AssemblyResult}.
3. Verify no more references: git grep -nE "use cor24_emulator::assembler|crate::assembler|::assembler::" should return zero hits.
4. cargo test --workspace passes; cargo clippy --workspace -- -D warnings passes.

Note: clippy may need fixes for two pre-existing warnings (the assembler.rs collapsible_match goes away naturally; the state.rs:621 collapsible_match remains and may need a tiny fix — collapse the if into the outer match arm, two-line change).

Done when:
- src/assembler.rs absent.
- No references to crate::assembler or cor24_emulator::assembler anywhere.
- cargo test --workspace passes.
- cargo clippy --workspace passes (-D warnings if you have time to fix the state.rs warning; otherwise note as pre-existing for step 009).

Next step: --next-slug final-sweep — grep verification, --help inventory, finished-baseline-vs-current diff, and summary doc; signal mike for relay.