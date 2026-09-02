# Client-Side Routing Overview

Velo includes a built-in client-side router (part of the unified `velo` crate: `Router`, `Route`, `Link`, `FRouter`) designed specifically for Single Page Applications running in WebAssembly.

---

## 1. How the Router Works

* **HTML5 History API**: Uses `window.history.pushState` to transition URLs without full browser page reloads.
* **Reactive Route Signals**: The current path and query strings are backed by reactive signals (`CURRENT_PATH`, `CURRENT_QUERY`). When the URL changes (via `<Link>` clicks or browser back/forward buttons), the router effect re-evaluates and switches views surgically.
* **Pattern Matching**: Supports exact paths (`/dashboard`), dynamic path parameters (`/users/:id`), and catch-all wildcard routes (`/**`).

---

## 2. Lazy-Loading Heavy Sections (`use_dynamic`)

`use_dynamic(loader, fallback)` is Velo's async lazy-loader primitive — the client-side analogue of
Next.js `next/dynamic` / React `lazy`. It shows the `fallback` (a loading placeholder) immediately,
then **swaps in** the real node once the async `loader` future resolves. This is the hook point for
route-based code splitting (5.P8): a heavy route section loads on demand instead of blocking first
paint.

```rust
use velo::prelude::*;

fn heavy_chart() -> DomNode {
    use_dynamic(
        || async {
            velo::sleep(300).await;          // e.g. fetch a chunk / data
            view! {
                <div class="chart"><h2>"Chart"</h2></div>
            }
        },
        view! { <div class="chart">"Loading chart…"</div> },
    )
}

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <h1>"Dashboard"</h1>
            { heavy_chart() }   // placeholder first, real chart swaps in
        </div>
    }
}
```

Notes:
- `loader` is called once per mount; the resolved `DomNode` is moved into place (not rebuilt), so
  its inner reactivity keeps working after the swap.
- The placeholder and loaded content never stack — the placeholder is removed when the swap happens.
- See `examples/dynamic` for a full routing demo with active-state `<Link>`s.

---

## 3. Typed Navigation (`route_path!` & compile-checked `to`)

When you build with `app!`, the macro scans `src/app/` and derives a typed `paths` module of route
helpers (`paths::INDEX`, `paths::users_id("42")`, …). On top of that, 5.P9 brings `typedRoutes`
parity: a misspelt or non-existent route fails to **compile** instead of silently 404-ing.

### `route_path!(..)` — validated path literal

`route_path!` re-validates a literal path against the same compile-time route registry, and expands
to the `&'static str`:

```rust
use velo::prelude::*;

view! {
    // ok — `/`, `/typed`, `/users` all exist under src/app/
    <Link to={ route_path!("/") } label="Home" />
    <Link to={ route_path!("/typed") } label="Typed" />
}
```

A dynamic param matches any **one** segment, and a declared `[...rest]` catch-all matches the rest,
so `route_path!("/posts/hello-velo")` resolves against `src/app/posts/[slug]/page.rs`. A typo that
matches nothing produces a build error listing the available routes:

```text
error: route_path! failed: no route in `src/app/` matches "/nope"
  available routes:
      - /
      - /posts/:slug
      - /typed
      - /users
      - /users/:id
```

`route_path!` is a path-only helper (`/posts/hello-velo`), not a path *builder* — to build a route
with a runtime value use `paths::*` (e.g. `paths::users_id(id)` where `id: &str`).

### Compile-checked `<Link to>` literals

In an `app!` crate, a string-literal `to="/..."` is validated the same way. `<Link to="/nope" />`
fails to build with `invalid 'to' route: no route in 'src/app/' matches "/nope"`. Braced
expressions (`to={ paths::users_id("1") }`, `to={ route_path!(..) }`, dynamic strings) pass through
unchecked, since the `paths::*` builders are already typed and dynamic strings are inherently
unchecked.

> **Note:** validation only runs when an `src/app/` layout exists. Manual-router examples (no
> `src/app/`) keep working with arbitrary `to` literals.

### Programmatic navigation

`navigate_to(route_path!(..))` moves between routes in code with the same compile-time guard:

```rust
view! {
    <button class="go" on:click={ move |_| navigate_to(route_path!("/typed")) }>
        "Go to /typed"
    </button>
}
```

See `examples/typed-nav` for a full demo: active-state nav via `route_path!` and literal `to`,
`navigate_to`, `paths::*` builders, and typed `FRouter::use_param` reads on param pages.
