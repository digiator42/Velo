use dom::{mount_to_id, DomNode};
use r#macro::view;
use router::{Link, Router};
use velo_core::Signal;
use wasm_bindgen::prelude::*;
mod components;
use components::UserCard;

fn home_page() -> DomNode {
    view! {
        <div class="page home">
            <h1>"Welcome to Velo SPA"</h1>
            <p>"This is an ultra high performance desktop-grade client-side application running entirely in WebAssembly."</p>
        </div>
    }
}

fn profile_page() -> DomNode {
    let user_name = Signal::new("Guest".to_string());
    let show_card = Signal::new(false);

    let user_name_click = user_name.clone();
    let toggle_card = show_card.clone();

    view! {
        <div class="page profile">
            <h1>"User Account Settings"</h1>

            <button on:click={ move |_| { toggle_card.set(!toggle_card.get()); } }>
                "Toggle User Preview Card"
            </button>

            <button on:click={ move |_| { user_name_click.set("Alice".to_string()); } }>
                "Change Name"
            </button>

            <hr />

            {
                move || {
                    if show_card.get() {
                        return view! { <UserCard name={ user_name.clone() } role={ "Admin".to_string() } /> };
                    } else {
                        dom::DomNode::text("")
                    }
                }
            }
        </div>
    }
}

fn dashboard_page() -> DomNode {
    // Shared state managed cleanly in component scopes
    let count = Signal::new(0);

    let count_text = count.clone();

    view! {
        <div class="page dashboard">
            <h1>"Performance Analytics Dashboard"</h1>
            <div class="metric-box">
                <h3>"Reactive Counter Tracker"</h3>
                // Pass the dedicated text reader pointer here
                <h2>{ count_text.get() }</h2>

                // Pass the dedicated mutation pointer here and use a clear statement block
                <button on:click={ move |_| { count.set(count.get() + 1); } }>
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
                <Link to="/" label="Home Navigation" />
                <Link to="/dashboard" label="Dashboard System" />
                <Link to="/profile" label="Profile Management" />
            </nav>
            // <hr />
            // {
            //     Router::new(|path| {
            //         match path {
            //             "/" => home_page(),
            //             "/dashboard" => dashboard_page(),
            //             "/profile" => profile_page(),
            //             _ => view! { <h1>"404 - Engine Page Not Found"</h1> }
            //         }
            //     })
            // }
        </div>
    };

    mount_to_id("app", app_shell);
}
