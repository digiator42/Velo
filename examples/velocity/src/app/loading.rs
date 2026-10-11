use veloasm::prelude::*;

/// Global loading placeholder shown (via `app!`'s automatic Suspense wiring)
/// while a route is mounting, driven by `veloasm::route_loading()`.
#[loading]
pub fn loading() -> DomNode {
    view! {
        <div class="veloasm-loading">
            "Loading route…"
        </div>
    }
}

