# Quickstart Guide

Build your first interactive Velo SPA in under 5 minutes using the recommended
file-based `src/app/` workflow.

---

## 1. Scaffold the Project

Create `Cargo.toml` linking the `velo` crate (plus `wasm-bindgen` for the entry
point):

```toml
[package]
name = "my-velo-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
velo = { path = "../../crates/velo" }
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [] }
```

And an `index.html` entry shell (Trunk uses this as the build source of truth):

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My First Velo App</title>
    <style>
        body { font-family: system-ui, sans-serif; background: #0f172a; color: #f8fafc; padding: 2rem; }
        .card { background: #1e293b; padding: 1.5rem; border-radius: 12px; max-width: 400px; }
        button { background: #38bdf8; color: #0f172a; font-weight: bold; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }
        button:hover { background: #0284c7; color: white; }
    </style>
    <link rel="data-trunk" href="Cargo.toml" data-type="rust"/>
</head>
<body>
    <div id="app"></div>
</body>
</html>
```

---

## 2. Write the Application

Create the app root at `src/lib.rs` and one page at `src/app/page.rs`. Velo's
`app!` macro reads `src/app/` at compile time and turns each `page.rs` into a
route, generating the `<Router>` wiring for you:

```rust
// src/lib.rs
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();                     // scan src/app/ + build routes

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    let shell = view! {
        <div id="app">
            <Router routes={ velo_app::routes() } />
        </div>
    };
    mount(shell);
}
```

```rust
// src/app/page.rs — the "/" home page
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    // 1. Create a reactive signal with a combined read+write handle
    let count = signal!(0);

    // 2. Build the DOM structure with the view! macro
    view! {
        <div class="card">
            <h1>"Velo Quickstart"</h1>
            <p>"Current count: " <strong>{ count }</strong></p>
            <button on:click={ move |_| count.set(count.get() + 1) }>
                "Increment"
            </button>
        </div>
    }
}
```

---

## 3. Run the Development Server

Start the Trunk development server:

```bash
trunk serve --watch
```

Trunk compiles your Rust to WebAssembly, serves `index.html` at
`http://127.0.0.1:8080`, opens your browser, and **auto-reloads on every save**.
Velo's built-in dev error overlay shows a full Rust diagnostic (`file:line:col`)
right on the page when the build fails. See
[Dev Server, Error Overlay & HMR](dev-server-and-hmr.md) for the full loop.

---

## 4. Add a Second Page

Add `src/app/about/page.rs`:

```rust
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! { <h1>"About"</h1> }
}
```

`app!` automatically generates a typed helper in `velo_app::paths`, so you can
link to it with compile-time checking (`route_path!` is a typo-proof path
literal; `paths::*` are path *builders*). More in
[Typed Navigation](../routing/overview.md#3-typed-navigation-route_path--compile-checked-to).

```rust
use velo::prelude::*;

// inside a #[page] or #[layout] view:
<Link to={ route_path!("/about") } label="About" />
```

---

## 5. How It Works

* `{ count }` in the template subscribes directly to the `count` signal; the
  macro unwraps it without needing `.get()`.
* On click, `count.set(...)` notifies subscribers.
* Only the inner text node containing the count changes in the real browser
  DOM — the header, card, and button stay untouched.
