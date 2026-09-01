use velo::prelude::*;

/// Global not-found page, registered at the `/**` fallback route.
#[not_found]
pub fn not_found() -> DomNode {
    view! {
        <div class="not-found">
            <h1>"404 — page not found"</h1>
            <p>"The URL you tried doesn’t match any route."</p>
            <Link to={ paths::INDEX } label="Back home" />
        </div>
    }
}