use velo::prelude::*;

/// Persistent shell for the dashboard. The `clicks` counter's signal is
/// created when the shell first mounts; navigating routes must NOT reset it
/// (that's the M4 layout-persistence guarantee).
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let clicks = signal!(0);
    view! {
        <div class="dash-shell">
            <header>
                <span class="brand">"Velo Async Dashboard"</span>
                <button on:click={ move |_| clicks.set(clicks.get() + 1) }>
                    "shell clicks: " { clicks }
                </button>
                <nav>
                    <Link to={ paths::INDEX } label="Home" />
                    <Link to={ paths::BROKEN } label="Broken subtree" />
                </nav>
            </header>
            <main>
                { child }
            </main>
        </div>
    }
}