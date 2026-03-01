#!/bin/bash
# =============================================================================
# Rust → COR24 Full Pipeline Demo
# =============================================================================
#
# This script demonstrates the complete compilation pipeline:
#
#   Rust Source (.rs)
#        ↓  rustc --target wasm32-unknown-unknown
#   WASM Binary (.wasm)
#        ↓  wasm2cor24
#   COR24 Assembly (.s)
#        ↓  cor24-asm (assembler)
#   Machine Code (.bin) + Listing (.lst)
#        ↓  cor24-run (emulator)
#   Execution with LED output
#
# =============================================================================

set -e
cd "$(dirname "$0")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${CYAN}"
echo "============================================================================="
echo "  Rust → COR24 Full Pipeline Demo"
echo "============================================================================="
echo -e "${NC}"

# Build tools if needed
if [ ! -f target/release/wasm2cor24 ] || [ ! -f target/release/cor24-run ]; then
    echo -e "${YELLOW}Building tools...${NC}"
    cargo build --release --quiet
fi

# =============================================================================
echo -e "\n${GREEN}STEP 1: Rust Source Code${NC}"
echo "============================================================================="
echo -e "File: ${BLUE}examples/blink/src/lib.rs${NC}"
echo ""
cat examples/blink/src/lib.rs
echo ""

# =============================================================================
echo -e "\n${GREEN}STEP 2: Compile Rust → WASM${NC}"
echo "============================================================================="
echo -e "Command: ${YELLOW}cargo build --target wasm32-unknown-unknown --release${NC}"
echo ""

cd examples/blink
cargo build --target wasm32-unknown-unknown --release 2>&1 | grep -v "Compiling\|Finished" || true
cd ../..

WASM_FILE="examples/blink/target/wasm32-unknown-unknown/release/blink.wasm"
WASM_SIZE=$(stat -f%z "$WASM_FILE" 2>/dev/null || stat -c%s "$WASM_FILE" 2>/dev/null)
echo -e "Output: ${BLUE}$WASM_FILE${NC} (${WASM_SIZE} bytes)"
echo ""
echo "WASM binary (hex dump of first 64 bytes):"
xxd -l 64 "$WASM_FILE"

# =============================================================================
echo -e "\n${GREEN}STEP 3: Translate WASM → COR24 Assembly${NC}"
echo "============================================================================="
echo -e "Command: ${YELLOW}wasm2cor24 blink.wasm -o blink.s${NC}"
echo ""

./target/release/wasm2cor24 "$WASM_FILE" -o examples/blink/blink.s

ASM_SIZE=$(stat -f%z "examples/blink/blink.s" 2>/dev/null || stat -c%s "examples/blink/blink.s" 2>/dev/null)
echo -e "Output: ${BLUE}examples/blink/blink.s${NC} (${ASM_SIZE} bytes)"
echo ""
echo "COR24 Assembly:"
echo "---------------"
cat examples/blink/blink.s

# =============================================================================
echo -e "\n${GREEN}STEP 4: Assemble → Machine Code${NC}"
echo "============================================================================="
echo -e "Command: ${YELLOW}cor24-asm blink.s -o blink.bin${NC}"
echo ""

# Use cor24-run to assemble (it has built-in assembler)
./target/release/cor24-run --assemble examples/blink/blink.s examples/blink/blink.bin examples/blink/blink.lst 2>&1 || true

if [ -f examples/blink/blink.bin ]; then
    BIN_SIZE=$(stat -f%z "examples/blink/blink.bin" 2>/dev/null || stat -c%s "examples/blink/blink.bin" 2>/dev/null)
    echo -e "Output: ${BLUE}examples/blink/blink.bin${NC} (${BIN_SIZE} bytes)"
    echo ""
    echo "Machine code (hex):"
    xxd examples/blink/blink.bin
fi

if [ -f examples/blink/blink.lst ]; then
    echo ""
    echo -e "Listing file: ${BLUE}examples/blink/blink.lst${NC}"
    echo "----------------"
    cat examples/blink/blink.lst
fi

# =============================================================================
echo -e "\n${GREEN}STEP 5: Execute on COR24 Emulator${NC}"
echo "============================================================================="
echo -e "Command: ${YELLOW}cor24-run blink.bin${NC}"
echo ""

./target/release/cor24-run --run examples/blink/blink.s 2>&1

# =============================================================================
echo -e "\n${GREEN}SUMMARY: Complete Artifact Chain${NC}"
echo "============================================================================="
echo ""
echo "  Source Files:"
echo "  ├── examples/blink/src/lib.rs      (Rust source)"
echo "  └── examples/blink/Cargo.toml      (Rust manifest)"
echo ""
echo "  Intermediate Artifacts:"
echo "  ├── blink.wasm                     (WebAssembly binary)"
echo "  └── blink.s                        (COR24 assembly)"
echo ""
echo "  Final Artifacts:"
echo "  ├── blink.bin                      (COR24 machine code)"
echo "  └── blink.lst                      (Assembly listing)"
echo ""
echo "  Execution:"
echo "  └── LED output showing binary counter"
echo ""
echo -e "${CYAN}=============================================================================${NC}"
echo -e "${GREEN}Pipeline complete!${NC}"
echo -e "${CYAN}=============================================================================${NC}"
