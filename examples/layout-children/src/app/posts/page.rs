use velo::prelude::*;

use crate::components::*;

/// `/posts` — showcases named **children** composition: panels nested inside
/// panels, exactly like a layout chain nesting its `child` leaves.
#[page]
pub fn page() -> DomNode {
    view! {
        <div class="page">
            <h1>"Posts"</h1>
            <Panel title={ "A panel with named children".to_string() }>
                <p>
                    "'children' is just a " <code>"Vec<DomNode>"</code>
                    " parameter — nested nodes arrive in order."
                </p>
                <Panel title={ "Nested panels".to_string() }>
                    <p>
                        "Route layouts do the same thing one level up: the routed "
                        <code>"child"</code>
                        " is handed down the segment layout chain from root shell to "
                        <code>"posts/layout.rs"</code>
                        "."
                    </p>
                </Panel>
            </Panel>

            <Panel title={ "Same-chain navigation".to_string() }>
                <p class="post-list">
                    "The segment layout persists while navigating under "
                    <code>"/posts"</code>
                    " — its counter keeps ticking into a"
                    <Link to={ paths::posts_slug("panel-nesting") } label={ " post page" } />
                    "."
                </p>
            </Panel>
        </div>
    }
}