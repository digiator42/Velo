use core::Signal;

use dom::{DomNode, mount_to_id};
use r#macro::view;
use router::{Router, link};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::dashboard::monitor_page;
mod components;
mod dashboard;

fn home_page() -> DomNode {
    view! {
        <div class="page home">
            <h1>"Velo Engine Workspace v1.0"</h1>
            <p>"Welcome to your custom compiled, ultra-high-performance WebAssembly application framework environment."</p>
            <p>"Velo skips Virtual DOM tree comparison bottlenecks completely, enabling pure surgical state updates natively."</p>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    let app_shell = view! {
        <div id="app-container">
            <nav class="navbar">
                { link("/", "Home Navigation") }
                " | "
                { link("/dashboard", "Dashboard System") }
            </nav>
            <hr />
            {
                Router::new(|path| {
                    match path {
                        "/" => home_page(),
                        "/dashboard" => monitor_page(),
                        _ => view! { <h1>"404 - Engine Page Not Found"</h1> }
                    }
                })
            }
        </div>
    };

    mount_to_id("app", app_shell);
}
