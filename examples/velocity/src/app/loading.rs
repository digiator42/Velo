use velo::prelude::*;

/// Global loading placeholder shown (via `app!`'s automatic Suspense wiring)
/// while a route is mounting, driven by `velo::route_loading()`.
#[loading]
pub fn loading() -> DomNode {
    view! {
        <div class="velo-loading">
            "Loading route…"
        </div>
    }
}
