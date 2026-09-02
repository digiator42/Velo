//! `examples/async-dashboard` — exercises §5.P5 (M5): async data via
//! `create_resource` + `<Suspense>`, automatic per-route `loading.rs`
//! placeholders, and `<ErrorBoundary>`-style recovery so a panicking subtree
//! shows `error.rs` while the rest of the app (the layout counter) keeps
//! running.
//!
//! ```text
//! src/app/layout.rs        -> persistent shell (+ a live counter)
//! src/app/page.rs          -> "/"      Suspense demo with a delayed resource
//! src/app/broken/page.rs   -> "/broken"  page that panics on render
//! src/app/error.rs         -> global error boundary fallback
//! src/app/loading.rs       -> global loading placeholder
//! ```

use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    let shell = view! {
        <div id="app-container">
            <Router routes={ velo_app::routes() } />
        </div>
    };
    mount(shell);
}