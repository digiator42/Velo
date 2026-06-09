use dom::{DomNode, mount_to_id};
use r#macro::view;
use router::{FRouter, Link, Route, Router};
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

fn monitor() -> DomNode {
    // Easily extract out your dynamic route parameters safely!
    let cluster_id = FRouter::param("id").unwrap_or_default();
    let node_id = FRouter::param("new_id").unwrap_or_default();

    view! {
        <div class="page text-center">
            <h1>"Cluster Node Analytics Room"</h1>
            <p>"Monitoring Group Pipeline ID: " { cluster_id.clone() }</p>
            <p>"Live Streaming Endpoint Node: " { node_id.clone() }</p>
        </div>
    }
}

fn catch_all_fallback() -> DomNode {
    view! {
       <div class="page text-center">
           <h1>"404 - Page Not Found"</h1>
           <p>"The route you entered does not exist in this application."</p>
           <p>"Try navigating back to the Home page and exploring from there."</p>
       </div>
    }
}

pub fn run_app() {
    let routes = vec![
        Route {
            path: "/",
            component: home_page,
        },
        // Matches parameters /:id/ and /:new_id dynamically!
        Route {
            path: "/dashboard",
            component: monitor_page,
        },
        Route {
            path: "/dashboard/:id/live/:new_id",
            component: monitor,
        },
        // Catch all wildcard rule handles anything remaining safely
        Route {
            path: "/**",
            component: catch_all_fallback,
        },
    ];

    let app_shell = view! {
        <div id="app-container">
            <nav class="navbar">
                <Link to="/" label="Home Navigation" />
                <Link to="/dashboard" label="Dashboard System" />
            </nav>
            <hr />
            <Router routes={ routes } />
        </div>
    };

    mount_to_id("app", app_shell);
}
