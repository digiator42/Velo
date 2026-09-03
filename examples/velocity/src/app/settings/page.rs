use velo::prelude::*;

/// `/settings` — theme + profile controls. Reads the `ThemeState` and
/// `AuthState` from context, toggles dark mode with a `class:` binding, and
/// edits the user's display name with `bind:value` on a local `String` signal.
#[page]
pub fn page() -> DomNode {
    let theme = use_context::<crate::ThemeState>().expect("theme context");
    let auth = use_context::<crate::AuthState>().expect("auth context");

    let is_dark = theme.dark.clone();
    let collapsed = theme.collapsed.clone();
    // Local editable copy of the display name (String signal for bind:value).
    let display_name = signal!(auth.user.get().name.clone());

    view! {
        <div class="settings-page">
            <Head title="Settings · Velocity" />
            <h1>"Settings"</h1>

            <section class="settings-section">
                <h2>"Appearance"</h2>
                <label>
                    <input type="checkbox" bind:checked={ is_dark } />
                    " Dark mode"
                </label>
                <label>
                    <input type="checkbox" bind:checked={ collapsed } />
                    " Collapse sidebar"
                </label>
            </section>

            <section class="settings-section">
                <h2>"Profile"</h2>
                <p>"Editing as: " { display_name }</p>
                <input type="text" placeholder="Display name..." bind:value={ display_name } />
            </section>
        </div>
    }
}
