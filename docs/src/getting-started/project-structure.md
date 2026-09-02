# Recommended Project Structure

Here is the standard layout for a Velo WebAssembly application using the
recommended `src/app/` file-based routing:

```
my-velo-app/
├── Cargo.toml          # Rust package configuration and dependencies
├── index.html          # HTML entry shell loaded by Trunk
└── src/
    ├── lib.rs          # App root: velo::app!() + run_app() + mount(shell)
    └── app/            # File-based routes (scanned by the app! macro)
        ├── layout.rs       # Root layout wrapping every page
        ├── page.rs         # "/" home page
        ├── not-found.rs    # 404 fallback
        ├── error.rs        # Error boundary fallback for the root segment
        ├── loading.rs      # Suspense fallback for async sections
        ├── about/
        │   └── page.rs     # "/about"
        ├── posts/
        │   ├── layout.rs   # Segment layout for /posts/*
        │   ├── page.rs     # "/posts"
        │   └── [slug]/
        │       └── page.rs # "/posts/:slug" (param folder)
        └── components/     # Reusable UI components (plain fns / #[component])
            ├── mod.rs
            └── user_card.rs
```

---

## 1. How `src/app/` File Routing Works

Velo's `app!` macro reads `src/app/` at compile time and derives the route
table. The filename conventions map directly to routes:

| File/folder               | Route                                     |
|---------------------------|-------------------------------------------|
| `src/app/page.rs`         | `/` (index)                               |
| `src/app/about/page.rs`   | `/about`                                  |
| `src/app/posts/page.rs`   | `/posts`                                  |
| `src/app/posts/[slug]/page.rs` | `/posts/:slug` (dynamic param)       |
| `src/app/[...rest]/page.rs` | `/**` (catch-all)                       |
| `src/app/layout.rs`       | root layout wrapping all pages            |
| `src/app/not-found.rs`    | 404 fallback                              |
| `src/app/error.rs`        | error-boundary fallback for the segment   |
| `src/app/loading.rs`      | Suspense fallback for async sections      |

Each `page.rs` exports a `#[page] pub fn page() -> DomNode`; each `layout.rs`
exports a `#[layout] pub fn layout(child: DomNode) -> DomNode`. The macro also
emits a typed `paths` module (`velo_app::paths::*`) of route helpers.

---

## 2. `src/lib.rs`

The app root wires file routing to the `<Router>` and mounts the app:

```rust
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();

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

---

## 3. `index.html` Conventions

Trunk uses `index.html` as the source of truth for the web asset pipeline. You
can link global CSS styles, web fonts, and the Rust Cargo manifest:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Velo App</title>

    <!-- Link external CSS managed by Trunk -->
    <link rel="stylesheet" href="static/styles.css"/>

    <!-- Instruct Trunk to compile the local Rust package -->
    <link rel="data-trunk" href="Cargo.toml" data-type="rust"/>
</head>
<body>
    <div id="app"></div>
</body>
</html>
```

---

## 4. Prelude Imports

`use velo::prelude::*;` seeds your file with the common primitives:
* Reactivity: `signal!`, `signal_vec!`, `memo`, `create_effect`, `RwSignal`,
  `SignalVec`, `batch`, `provide_context`, `use_context`.
* DOM & Templating: `DomNode`, `document`, `mount`, `RenderDynamic`, `view!`,
  `component`.
* Routing: `Router`, `Route`, `Link`, `route_path!`, `navigate_to`, `paths`.
* Control flow & metadata: `Show`, `Head`, `class_names!`, `use_dynamic`.

For small single-page widgets that don't need `src/app/` routing, you can skip
the `app!` macro and mount a tree directly with `mount()` — the manual `Route`
table and `#[component]` APIs still work for those SPAs.
