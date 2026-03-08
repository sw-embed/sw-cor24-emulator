#!/bin/bash
# Demo: Count down from 5 to 1 with UART output and breakpoint debugging
# Runs count_down.lgo which prints "54321" to UART
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Build the debugger
echo "=== Building cor24-dbg ==="
cargo build -p cor24-cli --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1
DBG="$PROJECT_DIR/target/debug/cor24-dbg"
LGO="$PROJECT_DIR/tests/programs/count_down.lgo"

echo ""
echo "=== Running Count Down Demo ==="
echo "Program: count_down.s -> prints '54321' to UART"
echo ""

$DBG "$LGO" <<'EOF'
disas 0 14
break 0x0B
run
info
print r1
continue
delete all
run
uart
quit
EOF
