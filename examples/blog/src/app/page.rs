use velo::prelude::*;

/// Home page at `/`.
#[page]
pub fn page() -> DomNode {
    view! {
        <div class="page">
            <Head title="Home · Velo Blog" meta={ vec![("description".to_string(), "The Velo Blog home page".to_string())] } />
            <h1>"Welcome to the Velo Blog"</h1>
            <p>"This app is wired up by "<code>"velo::app!"</code>" — no route table."</p>
            <p>{"Routes come from the src/app/ folder tree. Check the Blog page for nested `[slug]` routes."}</p>
        </div>
    }
}