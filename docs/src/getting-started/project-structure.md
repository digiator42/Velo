# Recommended Project Structure

Here is the standard layout for scaling a Velo WebAssembly application:

```
my-velo-app/
├── Cargo.toml          # Rust package configuration and dependencies
├── index.html          # HTML entry shell loaded by Trunk
├── src/
│   ├── lib.rs          # App root, entry point #[wasm_bindgen(start)], routing
│   ├── components/     # Reusable UI components (buttons, cards, navigation)
│   │   ├── mod.rs
│   │   ├── nav.rs
│   │   └── user_card.rs
│   ├── pages/          # Full-page route views
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   ├── dashboard.rs
│   │   └── not_found.rs
│   └── state/          # Global signals, stores, and Context structs
│       ├── mod.rs
│       └── app_state.rs
└── static/             # Static assets (images, fonts, global CSS)
    └── styles.css
```

---

## 1. `index.html` Conventions

Trunk uses `index.html` as the source of truth for building your web asset pipeline. You can link global CSS styles, web fonts, and the Rust Cargo manifest:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Velo App</title>
    
    <!-- Link external CSS managed by Trunk -->
    <link rel="stylesheet" href="static/styles.css"/>

    <!-- Instruct Trunk to compile the local Rust package -->
    <link rel="data-trunk" href="Cargo.toml" data-type="rust"/>
</head>
<body>
    <div id="app"></div>
</body>
</html>
```

---

## 2. Prelude Imports

Importing `use velo::prelude::*;` seeds your file with all common primitives:
* Reactivity: `create_signal`, `create_memo`, `create_effect`, `Signal`, `SignalVec`, `RwSignal`, `batch`, `provide_context`, `use_context`.
* DOM & Templating: `DomNode`, `document`, `RenderDynamic`, `view!`, `component`.
* Routing: `Router`, `Route`, `Link`.
