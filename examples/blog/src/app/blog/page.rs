use velo::prelude::*;

/// Blog index at `/blog`.
#[page]
pub fn page() -> DomNode {
    view! {
        <div class="page">
            <h1>"Posts"</h1>
            <ul>
                <li><Link to={ paths::blog_slug("hello-velo") } label="Hello, Velo" /></li>
                <li><Link to={ paths::blog_slug("async-arrows") } label="Async arrows" /></li>
                <li><Link to={ paths::blog_slug("controlled-inputs") } label="Controlled inputs" /></li>
            </ul>
        </div>
    }
}