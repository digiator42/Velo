use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// A simple application theme shared via context.
#[derive(Clone)]
struct Theme {
    dark: bool,
}

/// A `#[component]` — the macro rewrites the return type to `DomNode`,
/// so the body can end in a `view! { ... }` tail expression.
#[allow(non_snake_case)]
#[component]
fn UserCard(name: String, active: bool) {
    view! {
        <li class:active={ active } class="card">
            <span>"User: " { name }</span>
        </li>
    }
}

/// Keyed reactive list demo, driven by a `SignalVec`.
fn roster_page() -> DomNode {
    // A reactive collection of users.
    let users = SignalVec::new(vec![
        User {
            id: 1,
            name: "Ada".into(),
        },
        User {
            id: 2,
            name: "Linus".into(),
        },
        User {
            id: 3,
            name: "Grace".into(),
        },
    ]);

    let users_for_btn = users.clone();
    let users_for_btn2 = users.clone();

    view! {
        <div class="page">
            <h2>"Roster (keyed list)"</h2>
            <button on:click={ move |_| users_for_btn.push(User { id: 99, name: "New".into() }) }>
                "Add"
            </button>
            <button on:click={ move |_| { users_for_btn2.remove(0); } }>
                "Remove first"
            </button>

            // `key = |u| u.id` makes the list reconciled by stable id (fine-grained).
            <ul>
                {
                    for u in users key = |u: &User| u.id {
                        <UserCard name={ u.name.clone() } active={ u.id % 2 == 0 } />
                    }
                }
            </ul>
        </div>
    }
}

#[derive(Clone)]
struct User {
    id: u32,
    name: String,
}

/// Theme toggle demo: reads the theme from context and toggles a class/style.
fn theme_page() -> DomNode {
    let (dark, set_dark) = create_signal(false);
    // Derive a reactive color string from the boolean signal.
    let color = create_memo({
        let d = dark.clone();
        move || if d.get() { "orangered" } else { "teal" }.to_string()
    });
    let dark_toggle = dark.clone();

    view! {
        <div class:dark={ dark_toggle } class="page">
            <h2>"Theme (class: + style: toggles + context)"</h2>
            <button on:click={ move |_| set_dark.set(!dark.get()) }>
                "Toggle theme"
            </button>
            // Reactive inline style bound to a derived signal.
            <p style:color={ color }>
                "This text color is reactive."
            </p>
            // Context demonstration: a nested component reads the theme.
            <ThemeBadge />
        </div>
    }
}

/// Reads the theme provided by an ancestor via context.
#[allow(non_snake_case)]
#[component]
fn ThemeBadge() {
    let theme = use_context::<Theme>();
    view! {
        <div class="badge">
            {
                move || match theme.clone() {
                    Some(t) => DomNode::text(if t.dark { "Dark mode on" } else { "Light mode on" }),
                    None => DomNode::text("No theme in context"),
                }
            }
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    // Provide a theme to the whole app via context.
    provide_context(Theme { dark: false });

    let app_shell = view! {
        <div id="app-container">
            <h1>"Velo — new features demo"</h1>
            { roster_page() }
            { theme_page() }
        </div>
    };

    mount(app_shell);
}
