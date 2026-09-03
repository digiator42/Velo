use velo::prelude::*;

/// Global error-boundary fallback for any route. Renders when a subtree
/// raises `boundary_fault` (e.g. the task-detail "task not found" path).
/// The rest of the app — layout, nav, signals — keeps running.
#[error]
pub fn error() -> DomNode {
    view! {
        <div class="recovered">
            <h1>"Something went wrong"</h1>
            <p>"The failed subtree was replaced by this fallback, but the rest of Velocity keeps running."</p>
            <Link to={ paths::INDEX } label="Back to dashboard" />
        </div>
    }
}
