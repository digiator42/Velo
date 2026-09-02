use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let shell = view! {
        <div id="app-container">
            <nav>
                <Link to={ paths::INDEX } label="Home" active_class="is-active" />
            </nav>
            <main>
                <Router routes={ velo_app::routes() } />
            </main>
        </div>
    };
    mount(shell);
}
