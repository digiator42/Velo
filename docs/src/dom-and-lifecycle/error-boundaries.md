# Error Boundaries

An **error boundary** catches a failure while rendering a subtree and shows a
fallback instead of killing the whole app. The rest of the page keeps working.

---

## 1. File-based `error.rs` (recommended)

In an `app!` app, drop an `error.rs` file into a segment directory and mark its
function with `#[error]`:

```rust
// src/app/error.rs  (or src/app/<segment>/error.rs)
use velo::prelude::*;

#[error]
pub fn error() -> DomNode {
    view! {
        <div class="recovered">
            <h1>"We recovered."</h1>
            <p>"The failed subtree was replaced by this fallback."</p>
            <Link to={ paths::INDEX } label="Back to dashboard" />
        </div>
    }
}
```

`app!` wraps **every** page it generates in an error boundary, so a fault raised
anywhere inside that page's rendering is caught and replaced by the **nearest**
`error.rs` in its segment chain. The layout shell and other routes keep running.

---

## 2. How a fault is raised

On `wasm32-unknown-unknown`, `panic = "abort"` means real `panic!`s can't be
unwound — a panic simply aborts the instance. So Velo provides an
**app-level fault signal** that flags the current boundary'd subtree as failed
without needing unwinding:

```rust
#[page]
pub fn page() -> DomNode {
    if thing_failed() {
        velo::boundary_fault("the underlying service is down")
    } else {
        view! { <div>"All good."</div> }
    }
}
```

`boundary_fault(message)` marks the nearest enclosing `error_boundary` as failed
and returns a throwaway fragment so it can be used directly as a return value.
On native targets, genuine unwinding panics inside the subtree are also caught.

---

## 3. Programmatic `error_boundary`

When you're not using `app!`, or you want a boundary around a specific subtree,
call `error_boundary` directly:

```rust
use velo::prelude::*;

let fallback = view! { <div class="error-fallback">"Something went wrong."</div> };
let subtree = velo::error_boundary(fallback, Box::new(|| {
    view! { <Dashboard /> }
}));
```

The `build` closure is invoked, and its output is returned **unless** a fault was
raised inside (or, on native, a panic unwound), in which case `fallback` is shown.

`error_boundary` boundaries are **nestable**: each pushes its own status signal
onto an internal stack, and a fault is consumed by the closest enclosing
boundary.

---

## 4. Default fallback

When a route defines no `error.rs`, `app!` falls back to the built-in
`default_error_fallback()` — a simple `Something went wrong rendering this
subtree.` pane. Provide your own `error.rs` to customise it.

---

## See also

- [`examples/async-dashboard`](../../examples) — a `broken/page.rs` that raises a
  `boundary_fault`, recovered by a global `error.rs`, proving the rest of the app
  survives.