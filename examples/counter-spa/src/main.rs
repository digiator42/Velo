use core::Signal;

use r#macro::view;

fn main() {
    let count = Signal::new(0);

    let my_ui = view! {
        <div class="card">
            <h1>"Value is: " { count.get() }</h1>
            <button on:click={ move |_| count.set(count.get() + 1) }>
                "Increment Button"
            </button>
        </div>
    };

    dom::document()
        .body()
        .expect("Document should have a body element")
        .append_child(&my_ui.raw_node)
        .expect("Failed to append UI to document body");
}
