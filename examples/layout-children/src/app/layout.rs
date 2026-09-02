use velo::prelude::*;

/// Root layout — a persistent shell around every route. The routed leaf
/// arrives as `child` (this is component `children` at the route level) and is
/// placed inside `<main>`. Thanks to M4, this shell is mounted ONCE: the
/// `clicks` signal survives navigation between routes — only the leaf swaps.
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let clicks = signal!(0);
    view! {
        <div class="app-shell">
            <header>
                <span class="brand">"Layout · Children"</span>
                <button class="shell-count" on:click={ move |_| clicks.set(clicks.get() + 1) }>
                    "shell count: " { clicks }
                </button>
                <nav>
                    <Link to={ paths::INDEX } label="Feed" />
                    <Link to={ paths::POSTS } label="Posts" />
                </nav>
            </header>
            <main>
                { child }
            </main>
        </div>
    }
}