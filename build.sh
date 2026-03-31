#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building clock_slide.wasm..."
cargo build --target wasm32-wasip1 --release

SRC="target/wasm32-wasip1/release/clock_slide.wasm"
DST="clock_slide.wasm"
cp "$SRC" "$DST"
ln -sfn clock_slide.wasm slide.wasm
ln -sfn clock_slide.json manifest.json

SIZE=$(wc -c < "$DST")
echo "Done: $DST (${SIZE} bytes)"

echo "Packing clock.vzglyd..."
rm -f clock.vzglyd
zip -X -0 -r clock.vzglyd manifest.json slide.wasm assets/
VZGLYD_SIZE=$(wc -c < clock.vzglyd)
echo "Done: clock.vzglyd (${VZGLYD_SIZE} bytes)"
echo "Run with:"
echo "  cargo run --manifest-path ../vzglyd/Cargo.toml -- --scene ../vzglyd-clock"
