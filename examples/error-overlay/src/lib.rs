use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// Minimal app whose only job is to exist for `trunk serve --watch`: break the
/// code to see the Velo dev error overlay, then fix it to watch it recover.
#[wasm_bindgen(start)]
pub fn main() {
    let (count, set_count) = create_signal(0i32);
    let shell = view! {
        <div class="card">
            <h1>"Overlay test bench"</h1>
            <p>"This app is intentionally tiny. Break it to see the overlay."</p>
            <button on:click={ move |_| set_count.set(count.get() + 1) }>
                "count: " { count }
            </button>
        </div>
    };
    mount(shell);
}