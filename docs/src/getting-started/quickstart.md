# Quickstart Guide

Build your first interactive Velo SPA in under 5 minutes.

---

## 1. Create Project Files

Create an `index.html` file in your project root:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My First Velo App</title>
    <style>
        body { font-family: system-ui, sans-serif; background: #0f172a; color: #f8fafc; padding: 2rem; }
        .card { background: #1e293b; padding: 1.5rem; border-radius: 12px; max-width: 400px; }
        button { background: #38bdf8; color: #0f172a; font-weight: bold; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }
        button:hover { background: #0284c7; color: white; }
    </style>
    <link rel="data-trunk" href="Cargo.toml" data-type="rust"/>
</head>
<body>
    <div id="app"></div>
</body>
</html>
```

---

## 2. Write the Application Logic

In `src/lib.rs`, import Velo's prelude and define a reactive counter component:

```rust
use velo::prelude::*;
use velo::mount_to_id;
use wasm_bindgen::prelude::*;

fn app() -> DomNode {
    // 1. Create a reactive signal with initial value 0
    let (count, set_count) = create_signal(0);
    
    // Clone handle for the click event handler closure
    let count_for_click = count.clone();

    // 2. Build the DOM structure with the view! macro
    view! {
        <div class="card">
            <h1>"Velo Quickstart"</h1>
            <p>"Current count: " <strong>{ count }</strong></p>
            <button on:click={ move |_| set_count.set(count_for_click.get() + 1) }>
                "Increment"
            </button>
        </div>
    }
}

// 3. Entry point called when WASM loads in browser
#[wasm_bindgen(start)]
pub fn main() {
    mount_to_id("app", app());
}
```

---

## 3. Run the Development Server

Start the Trunk development server:

```bash
trunk serve --open
```

Trunk will compile your Rust code to WebAssembly, start a local web server (usually at `http://127.0.0.1:8080`), open your default browser, and watch for file changes to provide hot reloading!

Add `--watch` if you want it to rebuild and reload the browser on every save. See [Dev Server, Error Overlay & HMR](dev-server-and-hmr.md) for the full dev loop, including Velo's on-page compile-error overlay.

---

## 4. How It Works

* `{ count }` in the template subscribes directly to the `count` signal. The macro automatically unwraps the value without needing `.get()`.
* When the button is clicked, `set_count.set(...)` notifies subscribers.
* Only the inner text node containing the count number changes in the real browser DOM — the header, container card, and button remain completely untouched.
