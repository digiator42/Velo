use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="Typed · typed-nav" />
            <h1>"route_path! against param routes"</h1>
            <p>
                "A literal path into a param route is valid: "
                <code>{ route_path!("/users/42") }</code>
                " resolves against "
                <code>"/users/[id]"</code>
                "."
            </p>
            <p>
                "So is a specific post slug into "
                <code>"/posts/[slug]"</code>
                ": "
                <code>{ route_path!("/posts/hello-velo") }</code>
                "."
            </p>
        </div>
    }
}
