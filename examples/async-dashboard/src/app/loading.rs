use velo::prelude::*;

/// Global loading placeholder shown (via `app!`'s automatic Suspense wiring)
/// while a route mounts. The placeholder swaps to the real content on the
/// next microtask when `velo::route_loading()` flips.
#[loading]
pub fn loading() -> DomNode {
    view! {
        <div class="velo-loading">
            "Loading route…"
        </div>
    }
}