# Router & Route Configuration

Velo's recommended routing is **file-based** via the `app!` macro. A lightweight
manual `Route` table still works for small SPAs that don't want `src/app/`.

---

## 1. File-Based Routing with `app!`

Drop the line `velo::app!();` at the top of `src/lib.rs`, create your pages
under `src/app/`, and let the macro turn them into routes:

```text
src/app/
├── layout.rs            # root layout (wraps every page)
├── page.rs              # "/"
├── about/page.rs        # "/about"
├── posts/page.rs        # "/posts"
└── posts/[slug]/page.rs # "/posts/:slug"
```

```rust
// src/lib.rs
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();                        // scan src/app/ + build routes

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    let shell = view! {
        <div id="app">
            <nav>
                <Link to={ route_path!("/") } label="Home" />
                <Link to={ route_path!("/about") } label="About" />
            </nav>
            <main>
                <Router routes={ velo_app::routes() } />
            </main>
        </div>
    };
    mount(shell);
}
```

Each page is just a function in its own file:

```rust
// src/app/about/page.rs
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! { <h1>"About Page"</h1> }
}
```

Naming convention:
* `page.rs` → the page component for its folder segment.
* `layout.rs` → a `#[layout] pub fn layout(child: DomNode) -> DomNode` wrapper.
* `not-found.rs` → the 404 fallback.
* `error.rs` / `loading.rs` → error-boundary / Suspense fallbacks.

The macro also builds a typed `paths` module of route helpers — see
[Typed Navigation](../routing/overview.md#3-typed-navigation-route_path--compile-checked-to).

---

## 2. Manual Route Table (for small / non-`app!` SPAs)

When you don't use `app!` (a single-page widget, or an early prototype), define
routes with the `routes!` macro or a `Vec<Route>`:

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

The equivalent manual vector:

```rust
use velo::Route;

let routes = vec![
    Route { path: "/",            component: home_page },
    Route { path: "/dashboard",   component: dashboard_page },
    Route { path: "/**",          component: not_found_page },
];
```

> **Note:** with a manual table there is no `app!` macro, so `route_path!` and
> compile-checked `<Link to>` aren't available — literal `to` / `paths` are just
> strings.

---

## 3. Mounting the `<Router>`

Pass the route list to `<Router routes={ ... } />`. With `app!`, hand it
`velo_app::routes()`; with a manual table, hand it your `routes!`/`Vec<Route>`:

```rust
let app_shell = view! {
    <div id="app-container">
        <nav>
            <Link to="/" label="Home" />
            <Link to="/dashboard" label="Dashboard" />
        </nav>
        <main>
            <Router routes={ routes } />
        </main>
    </div>
};

mount(app_shell);
```

> **Tip:** Prefer `mount()` (appends to `document.body()`) or `mount_at(target, app)` (appends to a specific element). Both return a `RootHandle` that can be dropped or `.unmount()`-ed to tear the app down. `mount_to_id` is deprecated — use `mount()` / `mount_at()`.

---

## 4. Route Matching

Routes are matched in order:

1. **Exact paths** — `/`, `/dashboard`
2. **Parameterized paths** — `/users/:id`, `/cluster/:cluster_id/node/:node_id`
3. **Wildcard** — `/**` matches anything and serves as a fallback 404

When navigating to a URL, the router evaluates each pattern and renders the
first matching component. With `app!`, this table is derived from your
`src/app/` folder structure, and dynamic parameters come from `[param]` folders
(e.g. `posts/[slug]/page.rs` → `/posts/:slug`).
