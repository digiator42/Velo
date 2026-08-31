use velo::prelude::*;
use wasm_bindgen::prelude::*;

/// Compile-check: a `Memo<u32>` must auto-unwrap inside `view! { { memo } }`
/// without any explicit `.get()`. If this fails to compile, the `ViewValue for
/// Memo` impl or the `signal_value!` usage path is broken.
fn memo_unwrap_page() -> DomNode {
    let (base, _set_base) = create_signal(42u32);
    let doubled = create_memo({
        let b = base.clone();
        move || b.get() * 2
    });

    // `doubled` is used once in the view below, so the macro's `move ||`
    // closure is the sole consumer — no move conflicts.
    view! {
        <div>
            // plain memo auto-unwrap in text position — MUST compile
            <p class="memo-check">"doubled = " { doubled.clone() }</p>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    let app = view! {
        <div>
            { memo_unwrap_page() }
        </div>
    };
    // The old mount_to_id is fine for this check; no wrapper issues to test here.
    velo_dom::mount_to_id("app", app);
}
