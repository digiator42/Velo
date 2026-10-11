# veloasm

[![crates.io](https://img.shields.io/crates/v/veloasm.svg)](https://crates.io/crates/veloasm)

Fine-grained reactive SPA framework for Rust → WebAssembly. No virtual DOM — state changes update the real DOM surgically.

> 📚 **Full documentation: [digiator42.github.io/Veloasm](https://digiator42.github.io/Veloasm/)**

## Install

```toml
[dependencies]
veloasm = "0.1"
```

```sh
cargo install veloasm-cli
veloasm new my-app
veloasm dev   # open http://localhost:8080
```

## Taste

```rust
use veloasm::prelude::*;

let count = signal!(0);   // Copy handle: get/set/update, zero clones

view! {
    <div>
        <h2>{ count }</h2>
        <button on:click={ () => count.update(|c| *c += 1) }>"Increment"</button>
    </div>
}
```

## Layout

- `crates/velo` — runtime (`veloasm` package): reactivity, DOM, router, fetch, error overlay.
- `crates/macro` — `view!`, `#[component]`, `routes!`, `#[route]`, `app!`.
- `crates/velo-cli` — `veloasm new` / `dev` / `build` (wraps Trunk).
- `examples/` — runnable apps, plus real-world submodules (`luxshield`, `cyberflow`). Clone with `git clone --recurse-submodules`.

## Status

Compiles to `wasm32-unknown-unknown`. See the [docs](https://digiator42.github.io/Veloasm/) for reactivity, routing, Suspense, and deployment.
