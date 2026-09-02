use velo::prelude::*;

/// A single post at `/posts/:slug` — shares the exact layout chain with
/// `/posts`, so both shells persist across the navigation (M4 leaf-only swap).
#[page]
pub fn page() -> DomNode {
    let slug = FRouter::param("slug").unwrap_or_else(|| "unknown".to_string());
    view! {
        <article class="page">
            <h1>{ format!("Post: {slug}") }</h1>
            <p>
                "This leaf is the only thing that swapped — the root shell "
                <em>"and"</em>
                " the posts segment layout stayed mounted (their counts survive)."
            </p>
            <p><Link to={ paths::POSTS } label={ "Back to posts" } /></p>
        </article>
    }
}