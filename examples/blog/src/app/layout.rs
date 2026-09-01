use velo::prelude::*;

/// Root layout — a persistent shell around every route.
///
/// `child` is the matched leaf subtree (already composed with any nested
/// segment layouts). Rendering it as a child of `<main>` places it inside the
/// shell; navigating between routes re-runs the matched page, so the shell
/// stays mounted.
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    view! {
        <div class="blog-shell">
            <header>
                <span class="brand">"Velo Blog"</span>
                <nav>
                    <Link to={ paths::INDEX } label="Home" />
                    <Link to={ paths::BLOG } label="Blog" />
                </nav>
            </header>
            <main>
                { child }
            </main>
        </div>
    }
}