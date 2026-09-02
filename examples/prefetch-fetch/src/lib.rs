//! `examples/prefetch-fetch` — exercises Velo's JS-style `fetch` helpers:
//! `velo::fetch`, `fetch_json::<T>`, and the `<Link prefetch />` prop that
//! warms a destination's payload on hover/focus.
//!
//! ```text
//! src/lib.rs   -> nav with a <Link prefetch />, Router over collected_routes()
//! /users       -> fetches a public JSON API with fetch_json::<Vec<User>> inside
//!                <Suspense>, showing the loading fallback until data arrives
//! /            -> home; the nav link to /users is prefetched on hover
//! ```
//!
//! The `json` feature is required for `fetch_json`; it is enabled for this
//! crate in `Cargo.toml`. Watch the Network tab: hovering (or tab-focusing)
//! the "Users" link issues an early, low-priority `fetch`, so navigating there
//! later resolves from the HTTP cache instantly.
//!
//! Note: the placeholder API is public third-party data (jsonplaceholder);
//! swap `USERS_URL` for your own endpoint.

use serde::Deserialize;
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// Public test endpoint. In a real app this would be your own route data URL —
/// the same URL both prefetched by `<Link prefetch />` and fetched on the page.
const USERS_URL: &str = "https://jsonplaceholder.typicode.com/users";

#[derive(Clone, Debug, Deserialize)]
struct User {
    name: String,
    #[serde(default)]
    email: String,
}

// =============================================================================
// Routes (inventory-collected via #[route] + collected_routes())
// =============================================================================

/// `/` — landing page with the prefetch-on-hover nav.
#[route("/")]
pub fn home_page() -> DomNode {
    view! {
        <div>
            <h1>"Velo prefetch + fetch"</h1>
            <p>"Hover or tab-focus the nav link below, then open the DevTools Network tab."</p>
            <p class="hint">
                "The link has "
                <code>"prefetch"</code>
                " set, so moving onto it fires an early fetch of the destination."
            </p>
        </div>
    }
}

/// A quick display card for a fetched user.
fn user_card(user: &User) -> DomNode {
    // Clone into owned Strings so `view!` renders them directly (no borrowed
    // reference captured in a `'static` closure).
    let name = user.name.clone();
    let email = user.email.clone();
    view! {
        <li>
            <strong>{ name }</strong>
            <div class="muted">{ email }</div>
        </li>
    }
}

/// `/users` — fetches the JSON API with `fetch_json::<Vec<User>>` inside
/// `<Suspense>` and swaps the list in once it resolves.
#[route("/users")]
pub fn users_page() -> DomNode {
    let resource = create_resource(|| async move {
        velo::fetch_json::<Vec<User>>(USERS_URL).await
    });

    let susp_loading = resource.clone();
    let susp_value = resource.clone();

    view! {
        <div>
            <h1>"Users"</h1>
            <div class="hint">
                "Loaded via "
                <code>"fetch_json::<Vec<User>>(USERS_URL)"</code>
                ". If you prefetched this link, it resolves from the HTTP cache."
            </div>
            <Suspense loading={ susp_loading.loading() }
                      fallback={ view! { <p class="muted">"Loading users…"</p> } }>
                { move || match susp_value.value() {
                    Some(Ok(users)) => {
                        let list = users.iter().map(user_card).collect::<Vec<_>>();
                        view! { <ul>{ list }</ul> }
                    }
                    Some(Err(_)) => view! { <p class="muted">"Failed to load users."</p> },
                    None => view! { <p class="muted">"Loading users…"</p> },
                } }
            </Suspense>
        </div>
    }
}

// =============================================================================
// Entry point
// =============================================================================

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let app = view! {
        <div id="app">
            <nav>
                <Link to="/" label="Home" active_class="is-active" />
                // `prefetch` (bare boolean prop) pre-warms /users on hover/focus
                // so the JSON above loads from cache on navigation.
                <Link to="/users" label="Users (prefetch)" prefetch active_class="is-active" />
            </nav>
            <main>
                <Router routes={ collected_routes() } />
            </main>
        </div>
    };
    mount(app);
}
