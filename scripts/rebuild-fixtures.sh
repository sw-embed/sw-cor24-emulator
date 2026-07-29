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
# Every committed .lgo must be reproducible by running its generator.
# Nothing here is hand-edited: a fixture that can only be produced by
# hand is a fixture that silently rots, and the next `make` overwrites
# the hand-work anyway.
#
# An earlier version of this script kept led_on, count_down and
# led_blink out of REBUILD, on the theory that they were MakerLisp as24
# artifacts to be preserved byte-for-byte. That was checked and is
# false: building the committed led_on.s with as24 *itself* yields the
# same 0x00 that cor24-asm does, not the committed 0x01. Those bytes
# were not reproducible from their source by any assembler — they were
# stale, from a .s that had been corrected without a rebuild. They are
# now regenerated like everything else.
#
# REBUILD is still a list rather than a *.s glob, so that adding a .s
# with no corresponding fixture is a deliberate act. blinky_s2 stays out
# of it because it genuinely has no .s in this repo.

REBUILD=(
    hello_world
    hello_uart
    led_on
    count_down
    led_blink
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
            echo "UNCHECKED: $lgo has a .s but is not in REBUILD — add it or say why"
            drift=1
        else
            echo "UNCHECKED: $lgo (no .s source in this repo; cannot be regenerated)"
        fi
    done
else
    echo "cor24-asm not on PATH — skipping tests/programs/ rebuild."
fi

# --- docs/research/ is reference material, never a build input --------
#
# The C sources under docs/research/asld24 (as24, ld24, longlgo) are
# MakerLisp's reference toolchain, kept for comparison and validation
# only. This script never builds them and never uses them to produce a
# committed artifact — build artifacts come from our Rust toolchain
# (cor24-asm). Run them by hand when you want to check our output
# against the reference; that is what they are for.
#
# docs/research/asld24/sieve.lgo is therefore left exactly as imported.
# cor24-asm cannot assemble sieve.s today because it uses `.comm`
# (`.comm _flags,8191`, a BSS/common symbol) which our assembler does
# not implement. Until it does, that fixture stays an imported
# reference artifact rather than something this script regenerates.

# --- every shipped .lgo must carry a G record -------------------------
#
# .lgo is "load and go". Without a trailing G record the makerlisp
# monitor loads the bytes and drops back to the prompt — the go never
# happens, so a fixture downloaded to real hardware does nothing.
#
# cor24-emu hides this: emulator.rs falls back to `unwrap_or(0)` when no
# G record is present, so any program starting at 0 runs anyway and the
# whole test suite stays green. That fallback is exactly why this went
# unnoticed for months, and why the check has to be here rather than in
# a #[test].
#
# docs/research/ is exempt: those are imported reference artifacts used
# for comparison against our toolchain, not programs we ship to
# hardware, and we do not regenerate them. sieve.lgo is the one that
# lands there — it enters at 0x93, so `cor24-emu --lgo sieve.lgo` needs
# an explicit --entry 0x93, which is exactly what its callers in
# integration_tests.rs and emulator.rs already pass.
#
# This check needs no toolchain, so it runs unconditionally.
echo "=== G-record check ==="
missing_g=0
while IFS= read -r lgo; do
    [[ -f "$lgo" ]] || continue
    case "$lgo" in
        docs/research/*)
            echo "EXEMPT: $lgo (imported reference artifact, not shipped)"
            continue
            ;;
    esac
    if ! grep -q '^G' "$lgo"; then
        echo "NO G RECORD: $lgo (loads but will not run on hardware)"
        missing_g=1
        drift=1
    fi
done < <(git ls-files '*.lgo')
[[ $missing_g -eq 0 ]] && echo "all shipped .lgo files carry a G record"

if [[ $drift -ne 0 ]]; then
    echo "FIXTURES DRIFTED — review and commit if intentional."
fi
exit $drift
