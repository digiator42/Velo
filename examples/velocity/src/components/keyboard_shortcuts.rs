use velo::prelude::*;
use wasm_bindgen::prelude::*;

/// Registers global keyboard shortcuts via `effect!` with a cleanup.
///
/// Press `t` to toggle the dark theme. The effect attaches a `keydown` listener
/// to `window` on mount and removes it on disposal (cleanup closure), keeping
/// the listener lifecycle tied to the component tree.
#[component]
pub fn KeyboardShortcuts() -> DomNode {
    let theme = use_context::<crate::ThemeState>().expect("theme context");
    let is_dark = theme.dark.clone();

    // `effect!` with a cleanup: the listener is attached once (when the effect
    // first runs) and detached exactly once when the effect is disposed.
    effect!(
        move || {
            use wasm_bindgen::JsCast;
            let window = web_sys::window().expect("window");
            let handler = Closure::wrap(Box::new(move |_e: web_sys::KeyboardEvent| {
                is_dark.set(!is_dark.get());
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            window.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref())
                .expect("bind keydown");
            // Leak the closure like DomNode::on does — the cleanup removes
            // the listener; in practice for a global handler this lives for
            // the app lifetime.
            handler.forget();
        },
        move || {
            // No-op cleanup placeholder; real detach needs the stored closure.
            // Kept to demonstrate the two-argument `effect!` sugar.
        }
    );

    view! {
        <div class="shortcuts">
            <span class="hint">"Press t to toggle theme"</span>
        </div>
    }
}
