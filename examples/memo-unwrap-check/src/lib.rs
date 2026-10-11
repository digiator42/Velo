use veloasm::prelude::*;
use wasm_bindgen::prelude::*;

/// Compile-check: a `Memo<u32>` must auto-unwrap inside `view! { { memo } }`
/// without any explicit `.get()`. If this fails to compile, the `ViewValue for
/// Memo` impl or the `signal_value!` usage path is broken.
fn memo_unwrap_page() -> DomNode {
    let (base, _set_base) = create_signal(42u32);
    let doubled = create_memo(move || base.get() * 2);

    // `doubled` is `Copy`, so the macro's `move ||` closure moves it freely.
    view! {
        <div>
            // plain memo auto-unwrap in text position — MUST compile
            <p class="memo-check">"doubled = " { doubled }</p>
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
    // The new mount() is fine for this check — no wrapper issues to test here.
    mount(app);
}

