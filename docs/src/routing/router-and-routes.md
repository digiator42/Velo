# Router & Route Configuration

---

## 1. Defining Routes with `routes!`

The `routes!` declarative macro provides a clean syntax for defining route tables:

```rust
use velo::prelude::*;

fn home_page() -> DomNode {
    view! { <h1>"Home Page"</h1> }
}

fn dashboard_page() -> DomNode {
    view! { <h1>"Dashboard Page"</h1> }
}

fn not_found_page() -> DomNode {
    view! { <h1>"404 — Page Not Found"</h1> }
}

let routes = routes! {
    "/"           => home_page,
    "/dashboard"  => dashboard_page,
    "/**"         => not_found_page,  // Wildcard catch-all fallback
};
```

### Manual Route Table (Alternative)

You can also build the route vector manually using the `Route` struct:

```rust
use velo::Route;

let routes = vec![
    Route { path: "/",            component: home_page },
    Route { path: "/dashboard",  component: dashboard_page },
    Route { path: "/**",         component: not_found_page },
];
```

---

## 2. Mounting the `<Router>`

Pass the route list to `<Router routes={ ... } />` in your app root:

```rust
use velo::prelude::*;
use velo::mount;

#[wasm_bindgen(start)]
pub fn main() {
    let routes = routes! {
        "/"          => home_page,
        "/dashboard" => dashboard_page,
        "/**"       => not_found_page,
    };

    let app_shell = view! {
        <div id="app-container">
            <nav>
                <Link to="/"        label="Home" />
                <Link to="/dashboard" label="Dashboard" />
            </nav>
            <main>
                <Router routes={ routes } />
            </main>
        </div>
    };

    mount(app_shell);
}
```

> **Tip:** Prefer `mount()` (appends to `document.body()`) or `mount_at(target, app)` (appends to a specific element). Both return a `RootHandle` that can be dropped or `.unmount()`-ed to tear the app down.

---

## 3. Route Matching

Routes are matched in order:

1. **Exact paths** — `/`, `/dashboard`
2. **Parameterized paths** — `/users/:id`, `/cluster/:cluster_id/node/:node_id`
3. **Wildcard** — `/**` matches anything and serves as a fallback 404

When navigating to a URL, the router evaluates each route pattern and renders the first matching `component`.
