//! `examples/typed-nav` — exercises 5.P9's compile-time typed navigation
//! (`typedRoutes` parity).
//!
//! The `app!` macro scans `src/app/` and derives a typed `paths` module of
//! route helpers. On top of that, 5.P9 adds `route_path!`, which compile-checks
//! a path literal against that same route registry, and `<Link to="..">`
//! literal validation, so a misspelt `to` fails to build instead of silently
//! 404-ing.
//!
//! ```text
//! src/app/layout.rs        -> persistent shell: nav uses route_path! + literal to
//! src/app/page.rs          -> "/"        home; navigate_to(route_path!(..)) on click
//! src/app/typed/page.rs    -> "/typed"   route_path! against a param route
//! src/app/users/page.rs    -> "/users"   links via paths::user_id builder
//! src/app/users/[id]/page.rs -> "/users/[id]" param route (use_path returns typed)
//! src/app/posts/[slug]/page.rs -> "/posts/[slug]" second param route
//! ```
//!
//! To see the compile-time guard in action, change any `route_path!(..)` or
//! literal `to=".."` below to a route that doesn't exist (e.g.
//! `route_path!("/nope")`) and rebuild — the build fails with the list of
//! available routes.

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
