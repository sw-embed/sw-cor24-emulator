Extract isa/ workspace member into the standalone sw-cor24-isa repo.

Brief: /disk1/github/softwarewrighter/devgroup/tools/briefs/dcemu-extract-isa.md.

Sibling repo already cloned at ../sw-cor24-isa (one-time setup done).

What to do (single step; mechanical):
1. Edit Cargo.toml workspace.members: drop "isa".
2. Edit Cargo.toml [dependencies] cor24-isa path: ./isa -> ../sw-cor24-isa.
3. Inspect cli/Cargo.toml for any direct dep on ./isa or ../isa; update path if found.
4. git rm -r isa/.
5. cargo build --workspace --release.
6. cargo test --workspace.
7. cargo clippy --workspace --all-targets --all-features -- -D warnings.
8. ./target/release/cor24-emu -V (smoke).
9. ./target/release/cor24-emu --demo --time 1 (smoke run).
10. Commit.

Out of scope (per brief):
- Source-level changes to .rs files (use cor24_isa::... stays).
- Changes to other repos (web-*, x-tinyc, x-assembler) — mike-coordinated batch.
- Refactors inside sw-cor24-isa.

Done when:
- All four cargo gates green (build/test/clippy/smoke).
- isa/ absent.
- One commit on pr/extract-isa with the brief-spec message.