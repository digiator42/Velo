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

---

## 2. Using Components in Markup

In the `view!` macro, any tag that starts with an **uppercase letter** is treated as a component call:

```rust
view! {
    <div class="team-roster">
        <UserCard name="Ada Lovelace".into() role="Lead Architect".into() />
        <UserCard name="Alan Turing".into() role="Senior Engineer".into() />
    </div>
}
```
