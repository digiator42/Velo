use core::Signal;
use dom::{mount_to_id, DomNode};
use r#macro::view;
use router::{link, Router};
use wasm_bindgen::prelude::*;

fn home_page() -> DomNode {
    view! {
        <div class="page home">
            <h1>"Welcome to Velo SPA"</h1>
            <p>"This is an ultra high performance desktop-grade client-side application running entirely in WebAssembly."</p>
        </div>
    }
}

fn dashboard_page() -> DomNode {
    // Shared state managed cleanly in component scopes
    let count = Signal::new(0);

    // Create unique, cheap pointer clones for each isolated reactive consumer scope
    let count_text = count.clone();
    let count_click = count.clone();

    view! {
        <div class="page dashboard">
            <h1>"Performance Analytics Dashboard"</h1>
            <div class="metric-box">
                <h3>"Reactive Counter Tracker"</h3>
                // Pass the dedicated text reader pointer here
                <h2>{ count_text.get() }</h2>

                // Pass the dedicated mutation pointer here and use a clear statement block
                <button on:click={ move |_| { count_click.set(count_click.get() + 1); } }>
                    "Surgical Increment"
                </button>
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    // This function will now run reliably under both Trunk and manual wasm-bindgen builds
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
                        "/dashboard" => dashboard_page(),
                        _ => view! { <h1>"404 - Engine Page Not Found"</h1> }
                    }
                })
            }
        </div>
    };

    mount_to_id("app", app_shell);
}
