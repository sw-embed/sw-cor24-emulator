#!/bin/bash
# Demo: Sieve of Eratosthenes benchmark
# Runs sieve.lgo (entry at 0x93) computing 1000 iterations of prime sieve
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Build the debugger
echo "=== Building cor24-dbg ==="
cargo build -p cor24-cli --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1
DBG="$PROJECT_DIR/target/debug/cor24-dbg"
LGO="$PROJECT_DIR/docs/research/asld24/sieve.lgo"

echo ""
echo "=== Running Sieve Demo ==="
echo "Program: sieve.lgo (entry 0x93) -> 1000 iterations of prime sieve"
echo ""

$DBG --entry 0x93 "$LGO" <<'EOF'
disas 0x93 10
run 500_000_000
uart
info
quit
EOF
