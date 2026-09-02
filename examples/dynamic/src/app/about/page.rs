use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="About · dynamic" />
            <h1>"About"</h1>
            <p>"This page loads instantly."</p>
        </div>
    }
}
