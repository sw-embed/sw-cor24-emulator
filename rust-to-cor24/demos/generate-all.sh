#!/bin/bash
# Generate all demo artifacts end-to-end
#
# For each demo:
#   1. Compile Rust → MSP430 assembly (rustc --target msp430-none-elf --emit asm)
#   2. Translate MSP430 → COR24 assembly (msp430-to-cor24)
#   3. Assemble + run in emulator (cor24-run)
#
# Prerequisites:
#   rustup toolchain install nightly
#   rustup target add msp430-none-elf --toolchain nightly
#   cargo build (in rust-to-cor24/)
#
# The start convention: each Rust file has #[no_mangle] fn start() which the
# translator finds automatically. No --entry flag needed anywhere.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TRANSLATOR_DIR="$SCRIPT_DIR/.."

# Build the translator and emulator tools
echo "Building msp430-to-cor24 and cor24-run..."
(cd "$TRANSLATOR_DIR" && cargo build --release --quiet)
TRANSLATE="$TRANSLATOR_DIR/target/release/msp430-to-cor24"
RUN="$TRANSLATOR_DIR/target/release/cor24-run"

DEMOS=(demo_blinky demo_add demo_uart_hello demo_countdown demo_button_echo demo_echo demo_echo_v2 demo_fibonacci demo_fibonacci_iter demo_nested demo_stack_vars demo_panic)
PASSED=0
FAILED=0

for demo in "${DEMOS[@]}"; do
    DEMO_DIR="$SCRIPT_DIR/$demo"
    echo
    echo "========================================"
    echo "  $demo"
    echo "========================================"

    # --- Step 1: Rust → MSP430 assembly ---
    echo "  [1/3] Compiling Rust → MSP430 assembly..."
    (cd "$DEMO_DIR" && rustup run nightly cargo rustc \
        --target msp430-none-elf \
        -Z build-std=core \
        --release \
        -- --emit asm 2>&1) || {
        echo "  FAILED: Rust compilation"
        FAILED=$((FAILED + 1))
        continue
    }

    # Find the .s file
    MSP430_S=$(find "$DEMO_DIR/target/msp430-none-elf/release/deps/" -name "*.s" | head -1)
    if [ -z "$MSP430_S" ]; then
        echo "  FAILED: No .s file found"
        FAILED=$((FAILED + 1))
        continue
    fi
    cp "$MSP430_S" "$DEMO_DIR/${demo}.msp430.s"
    echo "  → ${demo}.msp430.s"

    # Verify start label exists in MSP430 output
    if ! grep -q '\.globl.*start' "$DEMO_DIR/${demo}.msp430.s"; then
        echo "  WARNING: .globl start not found in MSP430 output"
    fi

    # --- Step 2: MSP430 → COR24 assembly ---
    echo "  [2/3] Translating MSP430 → COR24 assembly..."
    "$TRANSLATE" "$DEMO_DIR/${demo}.msp430.s" -o "$DEMO_DIR/${demo}.cor24.s" || {
        echo "  FAILED: MSP430 → COR24 translation"
        FAILED=$((FAILED + 1))
        continue
    }
    echo "  → ${demo}.cor24.s"

    # --- Step 3: Assemble + run in emulator ---
    echo "  [3/3] Assembling and running in emulator..."
    TIME_LIMIT=5
    # Blinky and button_echo run forever — limit them
    if [ "$demo" = "demo_blinky" ] || [ "$demo" = "demo_button_echo" ] || [ "$demo" = "demo_echo" ] || [ "$demo" = "demo_echo_v2" ]; then
        TIME_LIMIT=2
    fi
    "$RUN" --run "$DEMO_DIR/${demo}.cor24.s" \
        --dump --speed 0 --time "$TIME_LIMIT" \
        > "$DEMO_DIR/${demo}.log" 2>&1 || true
    echo "  → ${demo}.log"

    PASSED=$((PASSED + 1))
    echo "  OK"
done

echo
echo "========================================"
echo "  Results: $PASSED passed, $FAILED failed (of ${#DEMOS[@]} demos)"
echo "========================================"
echo
echo "Artifacts per demo:"
echo "  <demo>.msp430.s  — MSP430 assembly from rustc"
echo "  <demo>.cor24.s   — COR24 assembly (with 'la r0, start' + 'jmp (r0)' reset vector)"
echo "  <demo>.log       — Emulator output (registers, memory, I/O at halt)"
