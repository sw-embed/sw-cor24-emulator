//! Process-level tests asserting `--quiet` stream classification.
//!
//! Per `cor24-emu --help`: under `--quiet`, "UART TX as plain text on
//! stdout; logs to stderr". The loader's "Loaded N bytes from ..." and
//! "Patched ..." messages count as logs, not UART output, so they
//! must not appear on stdout in quiet mode (otherwise they pollute
//! pipeline consumers that expect only UART bytes there).
//!
//! Regression for `dcsno-bootstrap-snobol4-toolchain` brief: dcsno's
//! wrapper had to `grep -v '^Loaded '` because `--load-binary`
//! messages leaked onto stdout under `--quiet`.

use std::io::Write;
use std::process::{Command, Stdio};

fn cor24_emu() -> &'static str {
    env!("CARGO_BIN_EXE_cor24-emu")
}

/// Assemble a tiny program via `cor24-asm -` (PATH dependency, same
/// pattern as the inline tests in cli/src/run.rs).
fn asm_to_lgo_file(source: &str, out: &std::path::Path) {
    let mut child = Command::new("cor24-asm")
        .arg("-")
        .arg("-o")
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cor24-asm not on PATH; required by quiet_streams tests");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let output = child
        .wait_with_output()
        .expect("cor24-asm wait_with_output failed");
    assert!(
        output.status.success(),
        "cor24-asm failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Tiny program that copies the byte at 0x080000 to the UART data
/// register (0xFF0100) once, then halts in a self-branch loop.
const PROBE_SOURCE: &str = "
    la   r1, 0x080000
    lb   r0, 0(r1)
    la   r1, 0xFF0100
    sb   r0, 0(r1)
halt:
    bra  halt
";

#[test]
fn quiet_lgo_load_binary_keeps_loader_log_off_stdout() {
    let dir = tempdir();
    let lgo = dir.join("probe.lgo");
    let aux = dir.join("aux.bin");
    asm_to_lgo_file(PROBE_SOURCE, &lgo);
    std::fs::write(&aux, b"B").unwrap();

    let output = Command::new(cor24_emu())
        .args([
            "--lgo",
            lgo.to_str().unwrap(),
            "--load-binary",
            &format!("{}@0x080000", aux.to_str().unwrap()),
            "-n",
            "100",
            "--speed",
            "0",
            "--quiet",
        ])
        .output()
        .unwrap_or_else(|e| panic!("spawn cor24-emu failed: {e}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The UART-emitted byte 'B' must be on stdout (the only thing
    // there per the --quiet contract).
    assert!(
        stdout.contains('B'),
        "stdout missing UART output 'B':\nstdout={stdout:?}\nstderr={stderr:?}",
    );
    // The "Loaded N bytes from ... at 0x..." log must NOT pollute
    // stdout — that's the dcsno-reported leak.
    assert!(
        !stdout.contains("Loaded"),
        "stdout leaked loader log:\nstdout={stdout:?}",
    );
    // It belongs on stderr instead.
    assert!(
        stderr.contains("Loaded 1 bytes from"),
        "stderr should carry the loader log:\nstderr={stderr:?}",
    );
}

#[test]
fn quiet_lgo_patch_keeps_log_off_stdout() {
    let dir = tempdir();
    let lgo = dir.join("probe.lgo");
    asm_to_lgo_file(PROBE_SOURCE, &lgo);

    let output = Command::new(cor24_emu())
        .args([
            "--lgo",
            lgo.to_str().unwrap(),
            "--patch",
            "0x080000=0x42",
            "-n",
            "100",
            "--speed",
            "0",
            "--quiet",
        ])
        .output()
        .unwrap_or_else(|e| panic!("spawn cor24-emu failed: {e}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains('B'),
        "stdout missing UART output 'B':\nstdout={stdout:?}\nstderr={stderr:?}",
    );
    assert!(
        !stdout.contains("Patched"),
        "stdout leaked patch log:\nstdout={stdout:?}",
    );
    assert!(
        stderr.contains("Patched 0x080000"),
        "stderr should carry the patch log:\nstderr={stderr:?}",
    );
}

#[test]
fn non_quiet_keeps_loader_log_on_stdout_for_humans() {
    // Without --quiet, the contract is implicit — the loader log
    // stays on stdout (where humans read it interleaved with the
    // post-run summary). This pins that behavior so the --quiet
    // fix doesn't accidentally redirect the human-mode output too.
    let dir = tempdir();
    let lgo = dir.join("probe.lgo");
    let aux = dir.join("aux.bin");
    asm_to_lgo_file(PROBE_SOURCE, &lgo);
    std::fs::write(&aux, b"B").unwrap();

    let output = Command::new(cor24_emu())
        .args([
            "--lgo",
            lgo.to_str().unwrap(),
            "--load-binary",
            &format!("{}@0x080000", aux.to_str().unwrap()),
            "-n",
            "100",
            "--speed",
            "0",
        ])
        .output()
        .unwrap_or_else(|e| panic!("spawn cor24-emu failed: {e}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Loaded 1 bytes from"),
        "non-quiet should keep loader log on stdout:\nstdout={stdout:?}",
    );
}

/// Per-test scratch directory under target/ so files survive any
/// /tmp cleanup pressure and the tests can run in parallel without
/// stomping on each other.
fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("quiet_streams.{pid}.{n}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
