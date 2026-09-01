//! `examples/blog` — exercises the file-based `velo::app!` router (§4 of the
//! roadmap): a nested `/blog/[slug]` route, typed `paths` helpers feeding
//! `<Link to={ .. }>`, a root layout, and a global not-found page.
//!
//! The `src/app/` directory is read at compile time:
//! ```text
//! src/app/layout.rs          -> root layout (wraps every route)
//! src/app/page.rs            -> "/"
//! src/app/blog/page.rs       -> "/blog"
//! src/app/blog/[slug]/page.rs-> "/blog/:slug"
//! src/app/not-found.rs       -> "**"
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