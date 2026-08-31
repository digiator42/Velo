//! Demonstrates the JS-style arrow-closure sugar in the `view!` macro.
//!
//! Instead of a `move ||` / `move |_|` closure you can write `() => { .. }`
//! (event handlers inject a discarded `_evt` arg) or `{ () => .. }` for reactive
//! child closures. All forms expand to real Rust `move` closures at compile time.

use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// A tiny counter toggled through arrow-sugar event handlers.
fn counter_page() -> DomNode {
    let (count, set_count) = create_signal(0i32);

    // Clones to move into each closure.
    let inc = set_count.clone();
    let dec = set_count.clone();
    let reset = set_count.clone();

    view! {
        <div class="card">
            <h2>"Counter (event-handler arrow sugar)"</h2>

            // `() => { .. }` -> `move |_evt: web_sys::Event| { .. }`
            <button on:click={ () => { inc.update(|c| *c += 1); } }>
                "Increment"
            </button>

            // `() => expr` -> expression-bodied handler
            <button on:click={ () => dec.update(|c| *c -= 1) }>
                "Decrement"
            </button>

            // `(e) => { .. }` -> `move |e| { .. }` with the event bound to `e`.
            <button on:click={ (e) => {
                let _ = e.target(); // the web_sys::Event is in scope
                reset.set(0);
            } }>
                "Reset"
            </button>

            // `{ () => expr }` -> reactive child closure `move || expr`
            <p>
                "Count: " { () => count.get().to_string() }
            </p>
        </div>
    }
}

/// Reactive text derived via `{ () => .. }` sugar inside content braces.
fn derived_page() -> DomNode {
    let (a, set_a) = create_signal(3i32);
    let (b, _set_b) = create_signal(4i32);
    let inc_a = set_a.clone();

    // `move` arrows need a fresh clone per closure, exactly like `move || ..`.
    let a1 = a.clone();
    let a2 = a.clone();
    let b2 = b.clone();

    view! {
        <div class="card">
            <h2>"Derived values (reactive arrow any-place)"</h2>

            <button on:click={ () => inc_a.update(|v| *v += 1) }>
                "Bump A"
            </button>

            <p>"A = " { () => a1.get() } ", B = 4"</p>

            // Expression-bodied reactive closure.
            <p>"Product = " { () => a2.get() * b2.get() }</p>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    let app = view! {
        <div id="app-container">
            <h1>"Velo — arrow-closure sugar"</h1>
            { counter_page() }
            { derived_page() }
        </div>
    };
    mount(app);
}
