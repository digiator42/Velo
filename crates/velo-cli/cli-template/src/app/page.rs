use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! {
        <div class="page">
            <h1>"Welcome to Velo!"</h1>
            <p>"Your app is ready. Edit src/app/page.rs to get started."</p>
            <p>"File-based routing: add a file at src/app/blog/page.rs to create /blog."</p>
            <p>"Run " <code>"velo dev"</code>" to start the dev server."</p>
        </div>
    }
}
