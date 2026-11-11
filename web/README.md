# unfold web

The browser version: a static SvelteKit site with the puzzle logic from `../core`, compiled to wasm.

```sh
pnpm dev     # builds the wasm, then serves at localhost:5173
pnpm build   # builds the wasm, then the static site into build/
pnpm test    # checks the wasm makes the same puzzles as native
pnpm check   # type checks
```

Needs Rust with the `wasm32-unknown-unknown` target, the `wasm-bindgen` CLI at the version pinned in
`../wasm/Cargo.toml`, and `wasm-opt` (binaryen).

Deployed on Cloudflare from `main`: root directory `web`, build command `sh scripts/ci-build.sh`,
output directory `build`, with `SKIP_DEPENDENCY_INSTALL=true` and `NODE_VERSION=26`.
`scripts/ci-build.sh` installs the toolchain on a fresh Linux machine and refuses to deploy a wasm
that makes different puzzles from native.
