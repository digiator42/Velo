use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// A simple application theme shared via context.
/// The theme holds a **signal** so that changes propagate reactively.
#[derive(Clone)]
struct Theme {
    dark: RwSignal<bool>,
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
    let next_id = std::cell::Cell::new(4u32);

    view! {
        <div class="page">
            <h2>"Roster (keyed list)"</h2>
            <button on:click={ move |_| { web_sys::console::log_1(&"PROBE roster-add clicked".into()); let id = next_id.get(); next_id.set(id + 1); users_for_btn.push(User { id, name: "New".into() }); } }>
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
    thread_local! { static C: std::cell::Cell<u32> = std::cell::Cell::new(0); }
    C.with(|c| { c.set(c.get()+1); web_sys::console::log_1(&format!("PROBE theme_page called count={}", c.get()).into()); });
    // Read the shared theme signal from context.
    let theme = use_context::<Theme>().expect("Theme context must be provided");
    let dark = theme.dark;
    // Derive a reactive color string from the boolean signal.
    let color = create_memo({
        let d = dark.clone();
        move || { let v = if d.get() { "orangered" } else { "teal" }.to_string(); web_sys::console::log_1(&format!("PROBE memo computed={}", v).into()); v }
    });
    let dark_toggle = dark.clone();

    view! {
        <div class:dark={ dark_toggle } class="page">
            <h2>"Theme (class: + style: toggles + context)"</h2>
            <button on:click={ move |_| { let next=!dark.get(); web_sys::console::log_1(&format!("PROBE click: setting dark={}", next).into()); dark.set(next); } }>
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

/// Async data demo: `create_resource` + reactive `Suspense`/`Show`.
///
/// `create_resource` returns a `Resource<T>` with a reactive `.loading()` bool.
/// `<Suspense>`/`<Show>` are *reactive* control-flow: built once, then
/// `reactive_switch` swaps fallback <-> content whenever the `loading` signal
/// flips — so the async resource automatically swaps in the resolved data.
fn async_page() -> DomNode {
    let resource = create_resource(|| async {
        // Simulate a network fetch that takes ~1.5 seconds.
        gloo_timers::future::TimeoutFuture::new(1500).await;
        42u32
    });

    // The loading predicate and the content's `{ value }` are each captured by
    // their own `move` closure, so clone the resource handle for each use.
    let susp_loading = resource.clone();
    let susp_value = resource.clone();
    let show_loading = resource.clone();
    let show_value = resource.clone();

    view! {
        <div class="page">
            <h2>"Async data (create_resource + Suspense/Show)"</h2>
            <Suspense loading={ susp_loading.loading() }
                      fallback={ view!{ <p class="muted">"Suspense: loading…"</p> } }>
                <p>"Suspense: loaded value = " { susp_value.value().unwrap_or(0) }</p>
            </Suspense>
            <Show when={ !show_loading.loading() }
                  fallback={ view!{ <em>"Show: still loading…"</em> } }>
                <p>"Show: value = " { show_value.value().unwrap_or(0) }</p>
            </Show>
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
                    Some(t) => DomNode::text(if t.dark.get() { "Dark mode on" } else { "Light mode on" }),
                    None => DomNode::text("No theme in context"),
                }
            }
        </div>
    }
}

/// A layout component that receives arbitrary **named children**.
/// `#[component]` generates a `PanelProps { title, children }` struct, and the
/// `view!` macro routes nested nodes into the `children` field, enabling
/// `<Panel title="..">{ .. }</Panel>` composition.
#[allow(non_snake_case)]
#[component]
fn Panel(title: String, children: Vec<DomNode>) {
    view! {
        <div class="panel">
            <h3 class="panel-title">{ title.clone() }</h3>
            <div class="panel-body">{ children }</div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("VELO PANIC: {}", info).into());
    }));
    run_app();
}

pub fn run_app() {
    // Provide a theme to the whole app via context.
    // Uses a signal so changes propagate reactively to all readers.
    provide_context(Theme { dark: signal(false) });

    let app_shell = view! {
        <div id="app-container">
            <h1>"Velo — new features demo"</h1>
            <Panel title={ "Named props & children".to_string() }>
                <p>"children passed by name"</p>
                <ThemeBadge />
            </Panel>
            { roster_page() }
            { theme_page() }
            { async_page() }
        </div>
    };

    // Keep the RootHandle alive for the lifetime of the app: dropping it (its
    // `Drop` impl) removes the mounted tree from the DOM.
    std::mem::forget(mount(app_shell));
}
