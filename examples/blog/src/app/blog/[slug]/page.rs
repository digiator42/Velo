use velo::prelude::*;

/// A single post at `/blog/:slug`.
///
/// The `[slug]` directory names the route param; values are read back with
/// `FRouter::param("slug")` (or the typed `FRouter::use_param`).
#[page]
pub fn page() -> DomNode {
    let slug = FRouter::param("slug").unwrap_or_else(|| "unknown".to_string());
    let title = format!("Post: {slug}");
    view! {
        <article class="post">
            <Head title={ title.clone() } meta={ vec![("description".to_string(), format!("Blog post {slug}"))] } />
            <h1>{ title }</h1>
            <p>"You are reading “" { slug } "”."</p>
            <p><Link to={ paths::BLOG } label={ "Back to posts" } /></p>
        </article>
    }
}