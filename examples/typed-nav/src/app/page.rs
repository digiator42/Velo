use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="Home · typed-nav" />
            <h1>"Compile-time typed navigation"</h1>
            <p>
                "This example exercises "
                <code>{ route_path!("/typed") }</code>
                " and "
                <code>"/users"</code>
                " as "
                <code>"to"</code>
                " props. Break any of them and the build fails at compile time."
            </p>
            <button
                class="go"
                on:click={ move |_| navigate_to(route_path!("/typed")) }
            >
                "Go to /typed (navigate_to route_path!)"
            </button>
        </div>
    }
}
