#!/bin/bash
# Demo: LED blink program showing LED toggling and UART output
# Runs led_blink.lgo which blinks LED D2 five times, printing 'L' each toggle
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Build the debugger
echo "=== Building cor24-dbg ==="
cargo build -p cor24-cli --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1
DBG="$PROJECT_DIR/target/debug/cor24-dbg"
LGO="$PROJECT_DIR/tests/programs/led_blink.lgo"

echo ""
echo "=== Running LED Blink Demo ==="
echo "Program: led_blink.s -> blinks LED D2 five times, prints 'L' each toggle"
echo ""

# Feed commands via stdin
$DBG "$LGO" <<'EOF'
disas 0 20
led
step 100
led
step 100
led
run 100000
uart
led
quit
EOF
