use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn main() {
    let (count, set_count) = create_signal(0i32);
    let shell = view! {
        <div class="card">
            <h1>"Overlay test bench"</h1>
            <p>"This app is intentionally tiny. Break it to see the built-in overlay."</p>
            <button on:click={ move |_| set_count.set(count.get() + 1) }>
                "count: " { count }
            </button>
        </div>
    };
    mount(shell);
}
