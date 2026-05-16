#!/bin/bash
# Demo: Blinky-S2 — LED D2 mirrors button S2
# Runs blinky_s2.lgo with S2 released (default) and pressed, dumping
# the LED/switch I/O register each time so you can see D2 follow S2.
#
# Toggling S2 *during* execution is exercised by the integration test
# `test_blinky_s2_led_follows_switch` (see tests/integration_tests.rs).
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building cor24-emu ==="
cargo build -p cor24-cli --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1
EMU="$PROJECT_DIR/target/debug/cor24-emu"
LGO="$PROJECT_DIR/tests/programs/blinky_s2.lgo"

run_case() {
    local label="$1"
    shift
    echo ""
    echo "=== $label ==="
    "$EMU" --lgo "$LGO" --time 1 --speed 100000 --dump "$@" 2>&1 \
        | grep -E "^  LED D2:|^  BTN S2:"
}

echo ""
echo "=== Program: blinky_s2.lgo (entry 0x0E) ==="
echo "    D2 follows S2: bit 0 of 0xFF0000 is switch on read, LED on write."
echo ""
cat "$LGO"

run_case "S2 released (default)"
run_case "S2 pressed (--switch on)" --switch on

echo ""
echo "=== Toggling S2 mid-run ==="
echo "The CLI takes a single --switch value at start-up. To toggle S2"
echo "during execution, run the integration test instead:"
echo ""
echo "    cargo test --test integration_tests test_blinky_s2_led_follows_switch"
