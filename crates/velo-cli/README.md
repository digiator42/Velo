# veloasm-cli

A lean CLI for [veloasm](https://github.com/digiator42/Velo) — a fine-grained reactive Rust→WASM SPA framework. Wraps [Trunk](https://trunkrs.dev/) for scaffolding, dev server, and production builds.

## Install

```sh
cargo install --path crates/velo-cli
```

Or build and use from the workspace:

```sh
cargo build -p veloasm-cli
./target/debug/Veloasm --help
```

## Commands

### `veloasm new <name>`

Scaffold a new veloasm app from the built-in template.

```sh
veloasm new my-app
```

Creates `examples/my-app/` with:

```
examples/my-app/
  index.html          # Trunk entry point (loads Cargo.toml as a Rust crate)
  Cargo.toml          # cdylib with veloasm dep
  src/
    lib.rs            # wasm entry + app shell (veloasm::app! + Router + mount)
    app/
      page.rs         # / route (file-based routing via #[page])
```

The template uses `veloasm::app!()` (file-based routing). Run `veloasm dev` to start hacking.

### `veloasm dev [name]`

Start the Trunk dev server with `--watch`.

```sh
veloasm dev my-app       # serve examples/my-app/index.html
veloasm dev              # serve ./index.html (current directory)
```

Runs `trunk serve --watch <index.html>`. Edit Rust/WASM source → Trunk recompiles and reloads. The built-in dev error overlay surfaces compile failures as an in-browser panel.

### `veloasm build [name]`

Build for production with `trunk build --release`.

```sh
veloasm build my-app     # build examples/my-app/index.html
veloasm build            # build ./index.html (current directory)
```

Output goes to `dist/`.

## Quick start

```sh
veloasm new my-app
cd examples/my-app
veloasm dev
# open http://localhost:8080
```

Edit `src/app/page.rs` — the browser hot-reloads on save. Add `src/app/blog/page.rs` to create `/blog` (file-based routing picks it up at compile time).

## Requirements

- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- Rust toolchain with `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)


