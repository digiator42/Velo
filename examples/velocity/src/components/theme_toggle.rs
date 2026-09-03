use velo::prelude::*;

/// A toggle button that flips the UI between light and dark themes.
///
/// Reads the `ThemeState` from context (provided at the app root) and flips
/// its `dark` signal on click. Demonstrates `provide!`/`context!` (context
/// consumption) and `effect!`-free reactive class binding via `class:`.
#[component]
pub fn ThemeToggle() -> DomNode {
    let theme = use_context::<crate::ThemeState>().expect("theme context");
    let is_dark = theme.dark.clone();

    view! {
        <button class="theme-toggle"
                class:dark={ is_dark }
                on:click={ move |_| is_dark.set(!is_dark.get()) }>
            { move || if is_dark.get() { "Light" } else { "Dark" } }
        </button>
    }
}
