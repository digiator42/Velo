use velo::prelude::*;

/// Root layout — a persistent shell around every route.
///
/// `child` is the matched leaf subtree (already composed with any nested
/// segment layouts). Rendering it as a child of `<main>` places it inside the
/// shell. Thanks to M4 layout persistence the shell is mounted ONCE: the
/// `clicks` counter below is a signal created when the shell first renders, so
/// it survives navigating between routes — only the leaf (`child`) is swapped.
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let clicks = signal!(0);
    view! {
        <div class="blog-shell">
            <header>
                <span class="brand">"Velo Blog"</span>
                <button class="layout-count" on:click={ move |_| clicks.set(clicks.get() + 1) }>
                    "layout count: " { clicks }
                </button>
                <nav>
                    <Link to={ paths::INDEX } label="Home" active_class="is-active" />
                    <Link to={ paths::BLOG } label="Blog" active_class="is-active" />
                </nav>
            </header>
            <main class={ class_names!(
                "page-main",
                (clicks.get() > 0).then_some("page-main--snapped"),
                (clicks.get() % 2 == 0).then_some("page-main--even"),
            ) }>
                { child }
            </main>
        </div>
    }
}