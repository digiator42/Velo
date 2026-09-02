# Fetching Data & Prefetching

---

## 1. The `velo::fetch` API

Velo provides three helpers in `velo::prelude::*` that wrap `window.fetch` with a
JS-style ergonomics feel:

| Function | Returns | Purpose |
|---|---|---|
| `fetch(url)` | `Result<VeloResponse, JsValue>` | Raw fetch; read body with `.text()` / `.json()` |
| `fetch_json::<T>(url)` | `Result<T, FetchError>` | Fetch **and** deserialize into `T` (typed) |
| `prefetch(url)` | `()` | Fire-and-forget background warm-up (used by `<Link prefetch />`) |

Both `fetch` and `fetch_json` are `async` and can be `.await`ed directly inside a
Velo `async () => {}` handler, or wrapped in `create_resource`.

```rust
use velo::prelude::*;

// Raw fetch: inspect status, then read the body.
on:click={ async () => {
    let resp = velo::fetch("/api/health").await.unwrap();
    if resp.ok() {
        log!("up: {}", resp.status()); // 200
    }
} }
```

### `VeloResponse`

The value returned by `fetch`. It is a light wrapper over `web_sys::Response`:

```rust
resp.status()          // u16 — HTTP status code
resp.ok()              // bool — true for 2xx
resp.status_text()     // String — e.g. "OK"
resp.url()             // String — final URL after redirects
resp.header("etag")    // Option<String> — a response header
resp.text().await      // Result<String, JsValue> — the full body as text
resp.json().await      // Result<JsValue, JsValue> — the raw parsed JSON value
```

### `fetch_json<T>` (typed)

For typed JSON you need Velo's optional **`json` feature** (default **off** to keep
the WASM core small). Enable it in your crate:

```toml
# Cargo.toml
[dependencies]
velo = { path = "../../crates/velo", features = ["json"] }
serde = { workspace = true, features = ["derive"] }
```

Then `#[derive(Deserialize)]` your type and decode with one call — the Rust analogue
of `await (await fetch(url)).json()`:

```rust
#[derive(serde::Deserialize)]
struct User { name: String, age: u8 }

let resource = create_resource(|| async {
    velo::fetch_json::<User>("/api/users/1").await.unwrap()
});
```

### `FetchError`

`fetch_json` returns a `FetchError` instead of a bare `JsValue`, so you can pattern-match:

```rust
match velo::fetch_json::<User>("/api/users/1").await {
    Ok(user)         => /* render */,
    Err(e)           => log!("{e}"),
}
```

The variants are `Network(String)` (network/CORS), `Status { code, reason }`
(non-2xx), and `Decode(String)` (body wasn't valid JSON for `T`).

---

## 2. `<Link prefetch />` — on-hover route warm-up

Passing the `prefetch` boolean prop to a `<Link>` pre-warms the destination's
payload **without** navigating:

- On `mouseenter` and `focus`, Velo issues a low-priority background `fetch(to)`.
- The result is **discarded** — it just lands in the browser's HTTP cache.
- When the user later clicks through and the page calls `fetch_json` (or `fetch`)
  on the same URL, the data resolves **instantly** from cache.

```rust
view! {
    <nav>
        // Bare `prefetch` is a boolean prop (same as `prefetch={true}`).
        <Link to="/users" label="Users" prefetch />
    </nav>
}
```

### Why it helps

Routing to a data-heavy page admits two latencies: the network round-trip for the
payload *and* the render. Prefetching overlaps the download with the user's
hover/focus, so the fetch that follows navigation is a cache hit.

Because prefetch is fire-and-forget and never blocks, an unpredicted visit just
falls back to the normal (uncached) fetch path — it never hurts.

> **Future hook point.** `velo::prefetch` is where Velo will also warm up the
> destination's per-`.wasm` chunk once real code-splitting lands. Today it warms
> only the raw payload.

---

## 3. End-to-end example

The full wiring lives in [`examples/prefetch-fetch`](../../../examples/prefetch-fetch/):
a `<Link prefetch />` in the nav plus a `/users` page that decodes a JSON API with
`fetch_json::<Vec<User>>` inside `<Suspense>`.

```rust
#[route("/users")]
pub fn users_page() -> DomNode {
    let resource = create_resource(|| async move {
        velo::fetch_json::<Vec<User>>(USERS_URL).await
    });
    let loading = resource.clone();
    let value = resource.clone();

    view! {
        <Suspense loading={ loading.loading() }
                  fallback={ view! { <p class="muted">"Loading users…"</p> } }>
            { move || match value.value() {
                Some(Ok(users)) => { /* render the list */ }
                Some(Err(_))    => view! { <p>"Failed to load."</p> },
                None            => view! { <p>"Loading…"</p> },
            } }
        </Suspense>
    }
}
```

Run it with `trunk serve` in the example directory and watch the DevTools Network
tab: hovering the "Users" link fires an early request, so the page's fetch is a
cache hit.