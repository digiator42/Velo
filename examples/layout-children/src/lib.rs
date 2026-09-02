//! `examples/layout-children` — proof that component **composition** and
//! **keyed lists** interoperate with owned, non-`Copy` props (5.P2), layered on
//! the file-based `app!` router:
//!
//! - `layout.rs` — root shell: the routed page arrives as a `child` DomNode
//!   (next.js-style `children`), persists across navigation (M4).
//! - `posts/layout.rs` — nested segment layout wrapping its own page child.
//! - `components.rs` — `PostCard`/`Panel` take **owned non-`Copy` props**
//!   (a `Post` struct with `String` fields) and named `children: Vec<DomNode>`.
//! - `page.rs` — a **keyed `for`** renders `<PostCard post={ p.clone() }>`
//!   where `p` is `&Post`; no `.get()` and, critically, no `Copy` bound.
//!
//! The `src/app/` tree:
//! ```text
//! src/app/layout.rs          -> root layout (persistent shell)
//! src/app/page.rs            -> "/"      (keyed feed)
//! src/app/posts/layout.rs    -> "/posts" segment layout
//! src/app/posts/page.rs -> "/posts" (panel children)
//! src/app/posts/[slug]/page.rs -> "/posts/:slug" (leaf-only swap)
//! ```

use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod components;

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