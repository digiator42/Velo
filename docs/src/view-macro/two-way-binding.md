# Two-Way Form Bindings (`bind:`)

Velo provides native two-way binding sugar via `bind:value` and `bind:checked` to eliminate `web_sys` event parsing boilerplate.

---

## 1. Text Inputs (`bind:value`)

Bind any `RwSignal<String>` or `WriteSignal<String>` directly to an `<input>`, `<textarea>`, or `<select>`:

```rust
use velo::prelude::*;

#[component]
fn SearchBox() {
    let query = signal(String::new());

    view! {
        <div class="search-container">
            <input
                type="text"
                placeholder="Type something..."
                bind:value={ query }
            />
            <p>"You typed: " { query }</p>
        </div>
    }
}
```

### What Happens Under the Hood:
1. **DOM → Signal**: Listens to the `input` event on the element and automatically writes `element.value` into `query.set(...)`.
2. **Signal → DOM**: Automatically binds the signal to the element's `value` attribute, keeping it synchronized when updated programmatically.

---

## 2. Checkboxes & Radio Buttons (`bind:checked`)

Bind boolean signals to checkboxes:

```rust
#[component]
fn ToggleAgreement() {
    let agreed = signal(false);

    view! {
        <label>
            <input
                type="checkbox"
                bind:checked={ agreed }
            />
            " I agree to the terms"
        </label>
    }
}
```

* Listens to the `change` event on the checkbox and sets the boolean signal.
* Automatically reflects programmatic updates to the checkbox in the UI.
