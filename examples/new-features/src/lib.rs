use velo::prelude::*;
use wasm_bindgen::prelude::*;

// =============================================================================
// Feature 1 — DomNode::empty()
//
// A documented alias for fragment(). Branches that aren't wrapped in their own
// view!{} can return DomNode::empty() instead of a placeholder text node.
// =============================================================================

#[derive(Clone, PartialEq)]
struct AppConfig {
    theme: &'static str,
    admin: bool,
}

fn show_secret(details: bool) -> DomNode {
    // `details` is a plain bool here; use a signal in real code.
    if details {
        view! { <div class="details">"Secret details"</div> }
    } else {
        DomNode::empty()
    }
}

// A reactive version reading from a signal. The closure is auto-wrapped by the
// view! macro via `render_expression` (requires `DomNode: RenderDynamic`).
fn reactive_secret(details: RwSignal<bool>) -> DomNode {
    let details = details.clone();
    view! {
        { move || {
            if details.get() {
                view! { <div class="details">"Reactive secret"</div> }
            } else {
                DomNode::empty()
            }
        } }
    }
}

// =============================================================================
// Feature 2 — Ergonomic macros: signal!, provide!, context!, effect!
// =============================================================================

fn macro_demo() -> DomNode {
    provide!(AppConfig {
        theme: "dark",
        admin: true,
    });

    let count = signal!(0);
    let config: Option<AppConfig> = context!();

    let tracking_count = count.clone();
    effect!(move || {
        // Tracks `count`; the read establishes the reactive dependency so the
        // effect reruns whenever the signal changes.
        let _ = tracking_count.get();
    });

    let cleanup_lifecycle = {
        let count_a = count.clone();
        let count_b = count.clone();
        (
            move || {
                let _ = count_a.get();
            },
            move || {
                let _ = count_b.get();
            },
        )
    };
    // The two-arg form runs `create_effect_with_cleanup`. Dropping the handle
    // (immediately here) runs the cleanup while the effect is being disposed.
    effect!(cleanup_lifecycle.0, cleanup_lifecycle.1);

    let theme = config.as_ref().map(|c| c.theme).unwrap_or_default();
    let admin = config
        .as_ref()
        .map(|c| c.admin.to_string())
        .unwrap_or_default();

    let label = view! {
        <p>
            "theme = " { theme }
            " admin = " { admin }
        </p>
    };

    let inc = {
        let count = count.clone();
        move || count.set(count.get() + 1)
    };

    view! {
        <div>
            { label }
            <button type="button" on:click={ move |_e| inc() }>
                { move || format!("count = {}", count.get()) }
            </button>
        </div>
    }
}

// =============================================================================
// Feature 3 — Inventory-collected routes via #[route] + collected_routes()
//
// Note: rustc does not support the key-value form `#[route = "..."]` for
// proc-macro attributes, so the macro accepts the parenthesized spelling
// `#[route("/path")]`. `route` is re-exported by `velo::prelude::*` above.
// =============================================================================

#[route("/users/:id")]
pub fn user_profile_page() {
    view! { <div>"User " { FRouter::param("id").unwrap_or_default() }</div> }
}

#[route("/about")]
pub fn about_page() {
    view! { <div>"About us"</div> }
}

#[route("/empty")]
pub fn empty_page() {
    DomNode::empty()
}

fn router_demo() -> DomNode {
    view! {
        <div>
            <nav>
                <Link to="/users/42" label="User 42" />
                <Link to="/about" label="About" />
                <Link to="/empty" label="Empty" />
                // `prefetch` (bare boolean prop) pre-warms `/about`'s payload
                // on hover/focus so navigation feels instant.
                <Link to="/about" label="About (prefetch)" prefetch />
            </nav>
            <Router routes={collected_routes()} />
        </div>
    }
}

// =============================================================================
// Entry point
// =============================================================================

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let app = view! {
        <div class="new-features">
            <h1>"New Features Demo"</h1>

            <section>
                <h2>"DomNode::empty()"</h2>
                { show_secret(false) }
                { reactive_secret(signal!(false)) }
            </section>

            <section>
                <h2>"Signal / Provide / Context / Effect macros"</h2>
                { macro_demo() }
            </section>

            <section>
                <h2>"Inventory-collected routes"</h2>
                { router_demo() }
            </section>
        </div>
    };
    mount(app);
}
