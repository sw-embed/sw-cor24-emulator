#!/usr/bin/env bash
# Rebuild committed .lgo fixtures from source and report drift.
#
# Two families are covered:
#
#   examples/i2c/*/, examples/spi/*/  — C demos, built by their Makefile
#                                       (tc24r + cor24-asm). Needs both tools.
#   tests/programs/                   — hand-written assembly, built directly
#                                       by cor24-asm. Needs cor24-asm only.
#
# Each family is skipped independently when its tools are missing, so a
# machine with only cor24-asm still drift-checks tests/programs/. Both
# families rebuild in place and `git diff --exit-code` against the
# committed .lgo, leaving any drift in the working tree for review.
#
# IMPORTANT — tests/programs/ is an allowlist, not a glob.
#
# Not every .lgo in tests/programs/ is a build product of the .s beside
# it. Several came from MakerLisp's reference as24 assembler and are
# preserved byte-for-byte on purpose; the .s files next to them are
# transcriptions, not the source of record. tests/integration_tests.rs
# pins that intent for led_on ("We preserve the .lgo as-is — it's from
# the reference toolchain"), whose fixture writes 0x01 where led_on.s
# writes 0x00. Rebuilding those would silently replace reference
# artifacts with cor24-asm output and break their tests.
#
# So REBUILD lists only the fixtures cor24-asm owns. Everything else is
# reported as PRESERVED and left alone. When you add a fixture that is a
# genuine cor24-asm build product, add its basename here.
#
# Preserved is NOT the same as untouched, though. Those fixtures carry a
# hand-appended G record — a pure text append that leaves every L record
# byte-identical. Preserving the reference *bytes* never required
# withholding the *entry point*; see the G-record check below.

REBUILD=(
    hello_world
    hello_uart
)

REPO_ROOT="$(git rev-parse --show-toplevel)" || exit 1
cd "$REPO_ROOT" || exit 1

have_tc24r=0
have_cor24asm=0
command -v tc24r >/dev/null 2>&1 && have_tc24r=1
command -v cor24-asm >/dev/null 2>&1 && have_cor24asm=1

drift=0

# --- examples/: C demos with Makefiles (tc24r + cor24-asm) -------------
if [[ $have_tc24r -eq 1 && $have_cor24asm -eq 1 ]]; then
    for dir in examples/i2c/*/ examples/spi/*/; do
        [[ -f "${dir}Makefile" ]] || continue
        echo "=== $dir ==="
        if ! (cd "$dir" && make clean && make); then
            echo "BUILD FAILED in $dir"
            drift=1
            continue
        fi
        if ! git diff --exit-code -- "${dir}"*.lgo; then
            echo "DRIFT in $dir"
            drift=1
        fi
    done
else
    [[ $have_tc24r -eq 0 ]] && echo "tc24r not on PATH — skipping examples/ rebuild."
    [[ $have_cor24asm -eq 0 ]] && echo "cor24-asm not on PATH — skipping examples/ rebuild."
fi

# --- tests/programs/: cor24-asm-owned fixtures only --------------------
if [[ $have_cor24asm -eq 1 ]]; then
    echo "=== tests/programs/ ==="
    for name in "${REBUILD[@]}"; do
        src="tests/programs/${name}.s"
        lgo="tests/programs/${name}.lgo"
        if [[ ! -f "$src" ]]; then
            echo "MISSING SOURCE: $src listed in REBUILD but not present"
            drift=1
            continue
        fi
        if ! cor24-asm "$src" -o "$lgo"; then
            echo "BUILD FAILED for $src"
            drift=1
            continue
        fi
        if ! git diff --exit-code -- "$lgo"; then
            echo "DRIFT in $lgo"
            drift=1
        fi
    done
    # Name every fixture this script did not rebuild. An unchecked
    # fixture must not read as a checked one just because it was quiet.
    for lgo in tests/programs/*.lgo; do
        [[ -f "$lgo" ]] || continue
        base="$(basename "$lgo" .lgo)"
        skip=0
        for name in "${REBUILD[@]}"; do
            [[ "$base" == "$name" ]] && skip=1 && break
        done
        [[ $skip -eq 1 ]] && continue
        if [[ -f "tests/programs/${base}.s" ]]; then
            echo "PRESERVED: $lgo (reference-toolchain artifact; .s is a transcription)"
        else
            echo "PRESERVED: $lgo (no .s source)"
        fi
    done
else
    echo "cor24-asm not on PATH — skipping tests/programs/ rebuild."
fi

# --- every tracked .lgo must carry a G record -------------------------
#
# .lgo is "load and go". Without a trailing G record the makerlisp
# monitor loads the bytes and drops back to the prompt — the go never
# happens, so a fixture downloaded to real hardware does nothing.
#
# cor24-emu hides this: emulator.rs falls back to `unwrap_or(0)` when no
# G record is present, so any program starting at 0 runs anyway and the
# whole test suite stays green. That fallback is exactly why this went
# unnoticed for months, and why the check has to be here rather than in
# a #[test]. sieve.lgo is the counter-example that proves it matters —
# it enters at 0x93, so before it had a G record `cor24-emu --lgo
# sieve.lgo` silently produced no output at all.
#
# This check needs no toolchain, so it runs unconditionally.
echo "=== G-record check ==="
missing_g=0
while IFS= read -r lgo; do
    [[ -f "$lgo" ]] || continue
    if ! grep -q '^G' "$lgo"; then
        echo "NO G RECORD: $lgo (loads but will not run on hardware)"
        missing_g=1
        drift=1
    fi
done < <(git ls-files '*.lgo')
[[ $missing_g -eq 0 ]] && echo "all tracked .lgo files carry a G record"

if [[ $drift -ne 0 ]]; then
    echo "FIXTURES DRIFTED — review and commit if intentional."
fi
exit $drift
