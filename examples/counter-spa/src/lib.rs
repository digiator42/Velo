use velo::prelude::*;
use wasm_bindgen::prelude::*;
mod components;
use components::{UserCard, UserCardProps};

// =============================================================================
// Pages
// =============================================================================

#[route("/")]
pub fn home_page() -> DomNode {
    view! {
        <div class="page home">
            <h1>"Welcome to Velo SPA"</h1>
            <p>"This is an ultra high performance desktop-grade client-side application running entirely in WebAssembly."</p>
        </div>
    }
}

#[route("/profile")]
pub fn profile_page() -> DomNode {
    // Combined read/write handles: `signal!` gives a Copy `RwSignal`, so there's
    // no (ReadSignal, WriteSignal) pair to split, no `.clone()` to move into
    // handlers, and no `.get()` needed to read.
    let user_name = signal!("Guest".to_string());
    let show_card = signal!(false);

    view! {
        <div class="page profile">
            <h1>"User Account Settings"</h1>

            // RwSignal::update mutates in place — no `.get()`/`.set()` round-trip.
            <button on:click={ async () => show_card.update(|c| *c = !*c) }>
                "Toggle User Preview Card"
            </button>

            <button on:click={ () => user_name.set("Alice".to_string()) }>
                "Change Name"
            </button>

            <hr />

            // `<Show when={ show_card }>` auto-unwraps the signal via
            // `signal_value!`, and `UserCard` receives a Copy RwSignal handle —
            // zero explicit `.get()` / `.clone()` anywhere in this view.
            <Show when={ show_card } fallback={ DomNode::text("") }>
                <UserCard name={ user_name } role="Admin" />
            </Show>
        </div>
    }
}

#[route("/dashboard")]
pub fn dashboard_page() -> DomNode {
    let count = signal!(0);

    view! {
        <div class="page dashboard">
            <h1>"Performance Analytics Dashboard"</h1>
            <div class="metric-box">
                <h3>"Reactive Counter Tracker"</h3>

                // `{ count }` auto-unwraps via ViewValue — no `.get()`.
                <h2>{ count }</h2>

                // `count` is Copy, so the click handler captures it with no
                // `.clone()`, and `update` reads/mutates in place.
                <button on:click={ () => count.update(|c| *c += 1) }>
                    "Surgical Increment"
                </button>

                // Async arrow handler: updates immediately, then `.await`s
                // `velo::sleep` before a second tick — `async () => {}` bodies
                // run through `wasm_bindgen_futures::spawn_local` automatically.
                <button on:click={ async () => {
                    count.update(|c| *c += 1);
                    velo::sleep(250).await;
                    count.update(|c| *c += 1);
                } }>
                    "Async Increment (+1, then +1 after 250ms)"
                </button>
            </div>
        </div>
    }
}

// =============================================================================
// Entry point
// =============================================================================

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    run_app();
}

pub fn run_app() {
    let app_shell = view! {
        <div id="app-container">
            <nav class="navbar">
                <Link to="/" label="Home" />
                <Link to="/dashboard" label="Dashboard" />
                <Link to="/profile" label="Profile" />
            </nav>

            // Routes are collected at compile time from the #[route] pages above.
            <Router routes={collected_routes()} />
        </div>
    };
    mount(app_shell);
}