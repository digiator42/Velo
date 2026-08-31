# Defining Components with `#[component]`

Components in Velo are pure Rust functions annotated with the `#[component]` attribute macro.

---

## 1. Writing a Component

```rust
use velo::prelude::*;

#[component]
pub fn UserCard(name: String, role: String) {
    view! {
        <div class="user-card">
            <h3>{ name }</h3>
            <span class="badge">{ role }</span>
        </div>
    }
}
```

### What `#[component]` Does:
* Rewrites the function signature to return `velo::DomNode`.
* Allows the function body to end directly with a `view! { ... }` tail expression without needing an explicit `return` statement.
* **Auto-generates a `<Name>Props` struct** with one `pub` field per parameter, rewrites the
  function to take that struct as a single `props` argument, and destructures it back into the
  original parameter names inside the body.
* Any param named `children: Vec<DomNode>` receives nested child markup (see
  [Children & Fragments](./children-and-fragments.md)).

> **Named props rule:** because calls are resolved against the generated `<Name>Props` struct,
> attribute keys must **match parameter names**, and props may be written **in any order**. The
> generated `NameProps` type must be in scope at the call site — it lives in the same module as the
> component, so `use components::{UserCard, UserCardProps};` (or a glob `use components::*;`).

---

## 2. Using Components in Markup

In the `view!` macro, any tag that starts with an **uppercase letter** is treated as a component call:

```rust
view! {
    <div class="team-roster">
        <UserCard name="Ada Lovelace".into() role="Lead Architect".into() />
        <UserCard role="Senior Engineer".into() name="Alan Turing".into() />
    </div>
}
```

Note the second `UserCard` writes `role` before `name` — the order does not matter, because props
are matched to the `UserCardProps` fields by name.
