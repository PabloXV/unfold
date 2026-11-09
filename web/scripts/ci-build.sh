#!/bin/sh
# Builds the site from a fresh Linux machine, such as Cloudflare Pages' builder: installs Rust
# with the wasm target and the exact wasm-bindgen and wasm-opt the project uses, builds, then
# refuses to deploy a wasm that doesn't make the same puzzles as native.
#
# Cloudflare Pages settings: root directory "web", build command "sh scripts/ci-build.sh",
# output directory "build".
set -eu
cd "$(dirname "$0")/.."

WASM_BINDGEN=0.2.130 # must match wasm-bindgen in wasm/Cargo.toml
BINARYEN=133
PNPM=12.18.0 # must match packageManager in package.json

tools="${TMPDIR:-/tmp}/unfold-tools"
mkdir -p "$tools/bin"
# ours first, ahead of any older pnpm the machine came with
export PATH="$tools/bin:$tools/node_modules/.bin:$HOME/.cargo/bin:$PATH"

if ! command -v rustup >/dev/null 2>&1; then
	curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
rustup target add wasm32-unknown-unknown

if [ "$(wasm-bindgen --version 2>/dev/null)" != "wasm-bindgen $WASM_BINDGEN" ]; then
	name="wasm-bindgen-$WASM_BINDGEN-x86_64-unknown-linux-musl"
	curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WASM_BINDGEN/$name.tar.gz" |
		tar xz -C "$tools/bin" --strip-components=1 "$name/wasm-bindgen"
fi

if ! command -v wasm-opt >/dev/null 2>&1; then
	curl -sSfL "https://github.com/WebAssembly/binaryen/releases/download/version_$BINARYEN/binaryen-version_$BINARYEN-x86_64-linux.tar.gz" |
		tar xz -C "$tools/bin" --strip-components=2 "binaryen-version_$BINARYEN/bin/wasm-opt"
fi

if [ "$(pnpm --version 2>/dev/null)" != "$PNPM" ]; then
	npm install --silent --prefix "$tools" "pnpm@$PNPM"
fi
echo "using pnpm $(pnpm --version), $(wasm-bindgen --version), $(wasm-opt --version)"
pnpm install --frozen-lockfile
pnpm build
node scripts/check-wasm.js
