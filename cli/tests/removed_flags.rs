//! Smoke tests for `--run` and `--assemble` removal.
//!
//! These flags were deleted when the in-tree assembler was split out
//! into the standalone `cor24-asm` binary. Invoking them must now
//! exit non-zero with a clear migration message, and `--help` must
//! not advertise them.

use std::process::Command;

fn cor24_emu() -> &'static str {
    env!("CARGO_BIN_EXE_cor24-emu")
}

#[test]
fn removed_flags_exit_nonzero_with_migration_message() {
    for removed in ["--run", "--assemble"] {
        let output = Command::new(cor24_emu())
            .arg(removed)
            .arg("prog.s")
            .output()
            .unwrap_or_else(|e| panic!("spawn {} failed: {}", cor24_emu(), e));
        assert!(
            !output.status.success(),
            "'{}' should exit non-zero, got {:?}",
            removed,
            output.status,
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("removed when the in-tree assembler was split"),
            "stderr for {} should explain removal:\n{}",
            removed,
            stderr,
        );
        assert!(
            stderr.contains("cor24-asm"),
            "stderr for {} should point at cor24-asm:\n{}",
            removed,
            stderr,
        );
    }
}

#[test]
fn help_advertises_lgo_not_run_or_assemble() {
    let output = Command::new(cor24_emu())
        .arg("--help")
        .output()
        .expect("spawn cor24-emu --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("--run "),
        "--help should not mention --run:\n{}",
        stdout,
    );
    assert!(
        !stdout.contains("--assemble"),
        "--help should not mention --assemble:\n{}",
        stdout,
    );
    assert!(
        stdout.contains("--lgo"),
        "--help should advertise --lgo:\n{}",
        stdout,
    );
}
