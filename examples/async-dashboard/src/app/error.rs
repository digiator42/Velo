use velo::prelude::*;

/// Global error-boundary fallback: rendered when any route below it panics.
#[error]
pub fn error() -> DomNode {
    view! {
        <div class="recovered">
            <h1>"We recovered."</h1>
            <p>"The broken subtree failed, but this fallback proves the app survived."</p>
            <Link to={ paths::INDEX } label="Back to dashboard" />
        </div>
    }
}