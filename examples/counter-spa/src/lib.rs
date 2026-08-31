use r#velo_macro::view;
use velo_core::create_signal;
use velo_dom::{mount, DomNode};
use velo_router::Link;
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
    // Split read/write signals: clean, explicit ownership.
    let (user_name, set_user_name) = create_signal("Guest".to_string());
    let (show_card, set_show_card) = create_signal(false);

    let show_toggle = show_card.clone();

    view! {
        <div class="page profile">
            <h1>"User Account Settings"</h1>

            <button on:click={ move |_| { set_show_card.set(!show_toggle.get()); } }>
                "Toggle User Preview Card"
            </button>

            <button on:click={ move |_| { set_user_name.set("Alice".to_string()); } }>
                "Change Name"
            </button>

            <hr />

            {
                // `user_name` is auto-unwrapped by the macro (no `.get()` needed)
                move || {
                    if show_card.get() {
                        view! { <UserCard name={ user_name.clone() } role={ "Admin".to_string() } /> }
                    } else {
                        velo_dom::DomNode::text("")
                    }
                }
            }
        </div>
    }
}

fn dashboard_page() -> DomNode {
    // Shared state managed cleanly in component scopes
    let (count, set_count) = create_signal(0);
    let count_display = count.clone();

    view! {
        <div class="page dashboard">
            <h1>"Performance Analytics Dashboard"</h1>
            <div class="metric-box">
                <h3>"Reactive Counter Tracker"</h3>
                <h2>{ count_display }</h2>
                <button on:click={ move |_| { set_count.set(count.get() + 1); } }>
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
        </div>
    };

    mount(app_shell);
}
