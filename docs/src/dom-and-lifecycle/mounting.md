# Mounting the Application

Mounting connects your root Velo `DomNode` hierarchy to the browser's live HTML DOM tree. The modern mounting API returns a `RootHandle` so you can tear the app down explicitly.

---

## 1. `mount()` — Mount into `document.body()`

The most common entry point. Appends your app's root to `<body>` and returns a `RootHandle`:

```rust
use velo::prelude::*;
use velo_dom::mount;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    let app_root = view! {
        <div class="app-root">
            <h1>"My Velo Application"</h1>
        </div>
    };

    let _handle = mount(app_root);

    // The app stays mounted as long as `_handle` is in scope.
    // Drop it (or call `_handle.unmount()`) to remove the app from the DOM.
}
```

There is **no wrapper element** — the root `DomNode` is appended as a child of `<body>` directly.

---

## 2. `mount_at()` — Mount into a Specific Container

Use this when you want to mount into a specific existing element (e.g. `<div id="app">`):

```rust
use velo::prelude::*;
use velo_dom::mount_at;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    let container = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("app")
        .expect("Velo: <div id=\"app\"> not found");

    let app_root = view! { <h1>"Hello, Velo!"</h1> };
    let _handle = mount_at(&container, app_root);
}
```

---

## 3. `RootHandle` — Teardown

`RootHandle` gives you explicit control over the mounted app:

```rust
let handle = mount(app_root);

// Later — explicit unmount (idempotent, safe to call multiple times):
handle.unmount();
```

When the `RootHandle` is **dropped**, the app is automatically removed from the DOM. This makes it perfect for tests, remounting, and hot-reload.

---

## 4. Legacy `mount_to_id()`

> **Deprecated:** prefer `mount()` or `mount_at()`. The old `mount_to_id("app", app)` form still works but cannot return a `RootHandle`, so tearing down requires manual DOM manipulation.

```rust
// Old style — avoid in new code
mount_to_id("app", app_root);
```

---

## 5. Accessing the Global Document (`document()`)

Velo exports a `document()` helper that returns a typed `web_sys::Document` instance:

```rust
use velo_dom::document;

let title = document().title();
document().set_title("New App Title");
```
