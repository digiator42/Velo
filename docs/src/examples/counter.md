# Example: Counter SPA

A complete, self-contained reactive counter application demonstrating signals, auto-unwrapping, and event handling.

---

## Source Code (`src/lib.rs`)

```rust
use velo::prelude::*;
use velo_dom::mount_to_id;
use wasm_bindgen::prelude::*;

#[component]
fn CounterApp() {
    let count = signal(0);
    
    let count_inc = count.clone();
    let count_dec = count.clone();
    let count_reset = count.clone();

    view! {
        <div class="counter-container">
            <h1>"Velo Counter"</h1>
            
            <div class="display">
                <span class="value">{ count }</span>
            </div>

            <div class="controls">
                <button on:click={ move |_| count_dec.update(|c| *c -= 1) }>
                    "- Decrement"
                </button>
                <button on:click={ move |_| count_reset.set(0) }>
                    "Reset"
                </button>
                <button on:click={ move |_| count_inc.update(|c| *c += 1) }>
                    "+ Increment"
                </button>
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    mount_to_id("app", CounterApp());
}
```

---

## Running with Trunk

```bash
trunk serve --open
```
