# Example: Counter SPA

A complete, self-contained reactive counter application demonstrating signals,
auto-unwrapping, and event handling.

---

## File-based form (`src/app/`)

```text
src/lib.rs
src/app/page.rs
```

**`src/lib.rs`** — app root with `app!` routing:

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

**`src/app/page.rs`** — the `"/"` page:

```rust
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    let count = signal!(0);
    view! {
        <div class="counter-container">
            <h1>"Velo Counter"</h1>
            <div class="display">
                <span class="value">{ count }</span>
            </div>
            <div class="controls">
                <button on:click={ move |_| count.update(|c| *c -= 1) }>
                    "- Decrement"
                </button>
                <button on:click={ move |_| count.set(0) }>
                    "Reset"
                </button>
                <button on:click={ move |_| count.update(|c| *c += 1) }>
                    "+ Increment"
                </button>
            </div>
        </div>
    }
}
```

The button closures each hold their own cheap `RwSignal` clone — no need to split
read/write handles.

---

## Single-page widget form (no `src/app/`)

If you don't need routing at all, mount a tree directly and skip `app!`:

```rust
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

fn app() -> DomNode {
    let count = signal!(0);
    view! {
        <div class="counter-container">
            <h1>"Velo Counter"</h1>
            <div class="display"><span class="value">{ count }</span></div>
            <div class="controls">
                <button on:click={ move |_| count.update(|c| *c += 1) }>"+"</button>
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    mount(app());
}
```

---

## Running with Trunk

```bash
trunk serve --watch --open
```
