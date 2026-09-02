# Suspense & Loading States

Velo provides two complementary loading mechanisms: per-route `loading.rs`
placeholders (emitted automatically by `app!`) and the on-demand `<Suspense>`
component for async data inside a page.

---

## 1. Per-route `loading.rs` (automatic)

Drop a `loading.rs` into a segment directory and mark its function with
`#[loading]`:

```rust
// src/app/loading.rs
use velo::prelude::*;

#[loading]
pub fn loading() -> DomNode {
    view! { <div class="velo-loading">"Loading route…"</div> }
}
```

`app!` shows this placeholder whenever a navigation to that segment is in
progress, driven by Velo's internal `route_loading()` signal. When the route
finishes mounting, the placeholder swaps to the real content on the next
microtask.

A `loading.rs` in a sub-directory only applies to routes under that segment; the
root `loading.rs` applies everywhere. In typical setups a single root
`loading.rs` is all you need to avoid a jarring blank flash between navigations.

---

## 2. `<Suspense>` for async data

When a page awaits async data (a fetch, a timer, a resource), wrap that part of
the tree in `<Suspense>` so a fallback shows while the data resolves:

```rust
#[page]
pub fn page() -> DomNode {
    let resource = create_resource(|| async {
        velo::sleep(600).await;
        42u32
    });

    let loading = resource.clone();
    let value = resource.clone();

    view! {
        <div class="page">
            <Suspense loading={ loading.loading() }
                      fallback={ view!{ <p class="muted">"Loading stats…"</p> } }>
                <p>"Health score = " { value.value().unwrap_or(0) } " / 100"</p>
            </Suspense>
        </div>
    }
}
```

How it works:

- `loading()` on a `create_resource` handle is a reactive `bool` signal — `true`
  while the future is pending.
- `<Suspense>` compiles down to a `reactive_switch()`: it shows `fallback` while
  `loading()` is true, and swaps in `content` (the children) when it flips false.
- The swap moves the **same** live DOM nodes in and out, so nested reactive
  expressions stay wired to their elements — children are never rebuilt.

`<Show>` and `<Suspense>` both compile to `reactive_switch`; `<Suspense>` is
just the loading-flavoured variant. You can also call `reactive_switch(when,
content, fallback)` directly for arbitrary two-branch reactive content.

---

## 3. Combining with lazy loading

`<Suspense>` pairs naturally with `use_dynamic` for a section that both loads
asynchronously and resolves to heavy content:

```rust
<Suspense loading={ r.loading() } fallback={ view! { <p>"Loading…"</p> } }>
    { use_dynamic(|| async {
        velo::sleep(400).await;
        view! { <Chart data={ data } /> }
    }) }
</Suspense>
```

---

## See also

- [`examples/async-dashboard`](../../examples) — a delayed resource feeding
  `<Suspense>` plus a root `loading.rs` shown during navigation.
- [Error boundaries](../dom-and-lifecycle/error-boundaries.md) — the companion
  `error.rs` fallback for failures, distinct from loading.
- [Lazy loading & code splitting](../routing/code-splitting-and-lazy-loading.md) —
  `use_dynamic` for on-demand subtrees.