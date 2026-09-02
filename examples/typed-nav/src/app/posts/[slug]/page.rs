use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    let slug = FRouter::param("slug").unwrap_or_else(|| "unknown".to_string());
    view! {
        <div>
            <Head title={ format!("Post {slug} · typed-nav") } />
            <h1>"Post"</h1>
            <p>"You are reading “" { slug } "”."</p>
        </div>
    }
}
