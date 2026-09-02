use velo::prelude::*;

#[layout]
pub fn layout(child: DomNode) -> DomNode {
    view! {
        <div class="shell">
            <nav>
                <Link to={ route_path!("/") } label="Home" active_class="is-active" />
                <Link to={ route_path!("/typed") } label="Typed" active_class="is-active" />
                <Link to="/users" label="Users" active_class="is-active" />
            </nav>
            <main>
                { child }
            </main>
        </div>
    }
}
