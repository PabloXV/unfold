#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
out=src/lib/wasm
want=$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' ../wasm/Cargo.toml)
if [ "$(wasm-bindgen --version)" != "wasm-bindgen $want" ]; then
	echo "need wasm-bindgen $want: cargo install wasm-bindgen-cli --version $want --locked" >&2
	exit 1
fi

cargo build --quiet -p unfold-wasm --release --target wasm32-unknown-unknown
wasm-bindgen ../target/wasm32-unknown-unknown/release/unfold_wasm.wasm --target web --out-dir "$out"
wasm-opt -Oz "$out/unfold_wasm_bg.wasm" -o "$out/unfold_wasm_bg.wasm"
