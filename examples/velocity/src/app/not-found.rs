use velo::prelude::*;

/// The 404 fallback. `app!` registers `not-found.rs` as the `/**` catch-all.
#[not_found]
pub fn not_found() -> DomNode {
    view! {
        <div class="not-found">
            <Head title="Not Found · Velocity" />
            <h1>"404"</h1>
            <p>"The page you're looking for doesn't exist."</p>
            <Link to={ paths::INDEX } label="Back to dashboard" />
        </div>
    }
}
