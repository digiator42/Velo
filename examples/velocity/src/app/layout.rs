use velo::prelude::*;
use crate::components::*;

/// Root layout wrapping every route. Renders the sidebar navigation and the
/// main content area; the matched route renders into the `{ child }` slot.
///
/// The sidebar links use compile-checked `paths::*` helpers and `<Link>`
/// with `prefetch`. The `class:` toggles for `dark`/`collapsed` react to the
/// `ThemeState` provided in `lib.rs`.
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let theme = use_context::<crate::ThemeState>().expect("theme context");
    let auth = use_context::<crate::AuthState>().expect("auth context");
    let is_dark = theme.dark.clone();
    let collapsed = theme.collapsed.clone();

    let _current = FRouter::use_route();

    view! {
        <div class="app-shell" class:dark={ is_dark.get() } class:collapsed={ collapsed.get() }>
            <aside class="sidebar">
                <div class="brand">"Velocity"</div>
                <nav class="nav">
                    <Link to={ paths::INDEX } label="Dashboard" prefetch
                          active_class="active" />
                    <Link to={ paths::SETTINGS } label="Settings" prefetch
                          active_class="active" />
                </nav>
                <div class="user">
                    "Signed in as "
                    { move || format!(" {}", auth.user.get().name) }
                </div>
                <ThemeToggle />
            </aside>
            <main class="main-content">{ child }</main>
        </div>
    }
}
