//! `examples/dynamic` — exercises 5.P8's `use_dynamic` async lazy-loader
//! primitive: a section renders a loading placeholder, then swaps in the real
//! component once an async loader resolves (the client-side analogue of
//! Next.js `next/dynamic`).
//!
//! ```text
//! src/app/layout.rs   -> persistent shell with nav (active-state <Link>s)
//! src/app/page.rs     -> "/"      lazy-loads a "heavy" chart section via use_dynamic
//! src/app/about/page.rs -> "/about" instant page, no lazy loading
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
        <div id="app">
            <Router routes={ velo_app::routes() } />
        </div>
    };
    mount(shell);
}
