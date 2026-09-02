use velo::prelude::*;

#[layout]
pub fn layout(child: DomNode) -> DomNode {
    view! {
        <div class="shell">
            <nav>
                <Link to={ paths::INDEX } label="Home" active_class="is-active" />
                <Link to={ paths::ABOUT } label="About" active_class="is-active" />
            </nav>
            <main>
                { child }
            </main>
        </div>
    }
}
