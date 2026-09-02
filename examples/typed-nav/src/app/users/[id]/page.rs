use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    let id = FRouter::use_param::<String>("id").unwrap_or_else(|| "?".into());
    view! {
        <div>
            <Head title={ format!("User {id} · typed-nav") } />
            <h1>"User"</h1>
            <p>"You are viewing user " <code>{ id }</code> "."</p>
        </div>
    }
}
