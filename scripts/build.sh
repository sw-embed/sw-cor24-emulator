#!/usr/bin/env bash
# Build the COR24 emulator library and CLI
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --workspace
cargo test --workspace
echo "Build and tests passed."
