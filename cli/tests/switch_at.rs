//! Process-level tests for `--switch-at`: scheduled S2 (button) changes
//! during a run, and the `[LED @ N]` change log at `--speed 0`.
//!
//! Together these let a golden-output test (reg-rs, demo.sh, or a plain
//! shell pipeline) assert that firmware *tracks* the switch over time,
//! not just that it read a value fixed at start-up with `--switch`. The
//! original motivation was sw-cor24-forth's `13-loop-switch-led.fth`
//! demo, which silently overflowed the data stack and halted mid-loop:
//! `--switch on` could not see that, because the LED was already right
//! before the CPU died.

use std::io::Write;
use std::process::{Command, Stdio};

fn cor24_emu() -> &'static str {
    env!("CARGO_BIN_EXE_cor24-emu")
}

/// Assemble a tiny program via `cor24-asm -` (PATH dependency, same
/// pattern as the other process-level tests).
fn asm_to_lgo_file(source: &str, out: &std::path::Path) {
    let mut child = Command::new("cor24-asm")
        .arg("-")
        .arg("-o")
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cor24-asm not on PATH; required by switch_at tests");
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

/// Copies the switch register bit into the LED register forever: the
/// LED mirrors S2 within a handful of instructions of any change.
const MIRROR_SOURCE: &str = "
    la   r1, 0xFF0000
loop:
    lbu  r0, 0(r1)
    sb   r0, 0(r1)
    bra  loop
";

fn mirror_lgo() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cor24-switch-at-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lgo = dir.join("mirror.lgo");
    asm_to_lgo_file(MIRROR_SOURCE, &lgo);
    lgo
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(cor24_emu())
        .args(args)
        .output()
        .expect("spawn cor24-emu");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The event/LED lines of a run, in order, as `(kind, instruction, state)`.
fn transitions(stdout: &str) -> Vec<(String, u64, String)> {
    stdout
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix('[')?;
            let (kind, rest) = rest.split_once(" @ ")?;
            let (at, rest) = rest.split_once("] ")?;
            let state = match kind {
                "S2" => rest.to_string(),
                "LED" => {
                    if rest.contains("D2 on") {
                        "on".into()
                    } else {
                        "off".into()
                    }
                }
                _ => return None,
            };
            Some((kind.to_string(), at.parse().ok()?, state))
        })
        .collect()
}

#[test]
fn led_follows_each_scheduled_switch_change() {
    let lgo = mirror_lgo();
    let (ok, stdout, stderr) = run(&[
        "--lgo",
        lgo.to_str().unwrap(),
        "--switch-at",
        "100:on,300:off,500:on",
        "--speed",
        "0",
        "-n",
        "700",
    ]);
    assert!(ok, "cor24-emu failed:\n{stderr}");
    let t = transitions(&stdout);
    let kinds: Vec<String> = t.iter().map(|(k, at, s)| format!("{k}@{at}:{s}")).collect();
    assert_eq!(
        kinds,
        [
            "S2@100:pressed",
            "LED@300:on",
            "S2@300:released",
            "LED@500:off",
            "S2@500:pressed",
            "LED@700:on",
        ],
        "unexpected transition sequence; stdout:\n{stdout}"
    );
}

#[test]
fn switch_at_events_are_applied_at_exact_instruction_counts() {
    // Events inside a single default batch (10 000 instructions at
    // --speed 0) must still land where asked: the batch is split at
    // each event.
    let lgo = mirror_lgo();
    let (ok, stdout, _) = run(&[
        "--lgo",
        lgo.to_str().unwrap(),
        "--switch-at",
        "7:on",
        "--switch-at",
        "9:off",
        "--speed",
        "0",
        "-n",
        "20",
    ]);
    assert!(ok);
    let s2: Vec<(u64, String)> = transitions(&stdout)
        .into_iter()
        .filter(|(k, _, _)| k == "S2")
        .map(|(_, at, s)| (at, s))
        .collect();
    assert_eq!(
        s2,
        [(7, "pressed".to_string()), (9, "released".to_string())]
    );
}

#[test]
fn quiet_mode_keeps_switch_log_off_stdout() {
    let lgo = mirror_lgo();
    let (ok, stdout, stderr) = run(&[
        "--lgo",
        lgo.to_str().unwrap(),
        "--switch-at",
        "100:on",
        "--speed",
        "0",
        "-n",
        "200",
        "--quiet",
    ]);
    assert!(ok);
    assert!(!stdout.contains("[S2 @"), "stdout polluted:\n{stdout}");
    assert!(stderr.contains("[S2 @ 100] pressed"), "stderr:\n{stderr}");
}

#[test]
fn rejects_malformed_switch_at() {
    let lgo = mirror_lgo();
    for bad in ["100", "100:maybe", "x:on", ""] {
        let (ok, _, stderr) = run(&[
            "--lgo",
            lgo.to_str().unwrap(),
            "--switch-at",
            bad,
            "-n",
            "1",
        ]);
        assert!(!ok, "--switch-at {bad:?} should be rejected");
        assert!(
            stderr.contains("--switch-at"),
            "stderr for {bad:?}:\n{stderr}"
        );
    }
}
