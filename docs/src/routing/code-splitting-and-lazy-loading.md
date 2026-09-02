# Lazy Loading & Code Splitting

Velo keeps the initial WASM bundle small by loading heavy sections on demand.
The primitive for this is **`use_dynamic`** — the client-side analogue of
Next.js `next/dynamic` / React `lazy`. A placeholder shows immediately, then the
real subtree swaps in once an async loader resolves.

> **Scope note:** Velo delivers code-splitting through the `use_dynamic`
> **primitive** rather than automatic per-`.wasm`-file splitting. Real
> `--split-linked-modules` route chunking is a future enhancement; `use_dynamic`
> is the stable hook point your app can build on today.

---

## 1. `use_dynamic(loader, fallback)`

```rust
use velo::prelude::*;

fn heavy_chart() -> DomNode {
    use_dynamic(
        || async {
            velo::sleep(1200).await;                  // "fetch the heavy data / module"
            view! { <div class="chart"><h2>"Heavy chart"</h2></div> }
        },
        view! { <div class="chart placeholder">"Loading heavy chart…"</div> },
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

How it works:

- Shows `fallback` immediately when the returned node attaches to the tree.
- Spawns the async `loader` (via `wasm_bindgen_futures::spawn_local`).
- When the future resolves, **swaps in** the returned `DomNode` — the placeholder
  is removed and the resolved node is moved into place.
- The resolved node is **moved**, not rebuilt, so its inner reactive expressions
  keep working after the swap.

---

## 2. Guidelines

- **`loader` runs once per mount.** The async work happens on the next microtask,
  so the placeholder always shows first for at least one frame.
- **Use it for genuinely heavy sections** — a chart, a long list, an
  infrequently-visited panel — not for markup that's already cheap.
- **Toggle visibility with `<Suspense>`** when the section should only appear
  after data arrives — see [Suspense & Loading states](suspense-and-loading.md).
- **The placeholder never stacks** with the loaded content; it's removed on swap.

---

## 3. Example

`examples/dynamic` is a full `app!` demo: `/` lazy-loads a "heavy chart" via
`use_dynamic` with a 1200ms delayed loader and a visible placeholder; `/about`
loads instantly from the shell. Both pages share a persistent `<layout>` with
active-state `<Link>`s, and navigating back re-runs the loader for a fresh swap
demonstration.

---

## See also

- [Routing overview](overview.md) — where `use_dynamic` sits alongside the router.
- [`examples/dynamic`](../../examples) — the runnable lazy-loading demo.