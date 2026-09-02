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
