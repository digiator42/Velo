use veloasm::prelude::*;

/// Home page at `/`.
#[page]
pub fn page() -> DomNode {
    view! {
        <div class="page">
            <Head title="Home · veloasm Blog" meta={ vec![("description".to_string(), "The veloasm Blog home page".to_string())] } />
            <h1>"Welcome to the veloasm Blog"</h1>
            <p>"This app is wired up by "<code>"veloasm::app!"</code>" — no route table."</p>
            <p>{"Routes come from the src/app/ folder tree. Check the Blog page for nested `[slug]` routes."}</p>
        </div>
    }
}
