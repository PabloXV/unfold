#!/bin/sh
# Builds unfold-core for the browser into src/lib/wasm: cargo compiles it to wasm, wasm-bindgen
# writes the JS that loads it, and wasm-opt shrinks it.
set -eu
cd "$(dirname "$0")/.."
out=src/lib/wasm

cargo build --quiet -p unfold-wasm --release --target wasm32-unknown-unknown
wasm-bindgen ../target/wasm32-unknown-unknown/release/unfold_wasm.wasm --target web --out-dir "$out"
wasm-opt -Oz "$out/unfold_wasm_bg.wasm" -o "$out/unfold_wasm_bg.wasm"
