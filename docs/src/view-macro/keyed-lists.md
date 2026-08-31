# Keyed Reactive Lists

When rendering collections of items, fine-grained list reconciliation ensures that adding, removing, or re-ordering items only alters the specific DOM elements affected, preserving input focus and DOM state.

---

## 1. Keyed `for` Loop Syntax

Use `{ for item in collection key = |item| key_expr { ... } }`:

```rust
use velo::prelude::*;

#[derive(Clone)]
struct User {
    id: u64,
    name: String,
}

let users = signal_vec(vec![
    User { id: 1, name: "Ada Lovelace".into() },
    User { id: 2, name: "Linus Torvalds".into() },
]);

view! {
    <ul>
        {
            for user in users key = |u: &User| u.id {
                <li>
                    <span>"ID: " { user.id }</span>
                    <span>" Name: " { user.name.clone() }</span>
                </li>
            }
        }
    </ul>
}
```

---

## 2. Why Keys Matter

1. **Reconciliation by Identity**: Velo compares the keys before and after a mutation. Items with existing keys are preserved and re-ordered in the DOM without re-creating nodes.
2. **Performance**: Only newly added keys create new DOM nodes; removed keys have their corresponding nodes detached and disposed.
3. **Closure Requirement**: The item render closure implements `Clone`, allowing it to capture shared state handles when rendering child components.

---

## 3. Static Lists (Non-Keyed)

For static slices or arrays that do not change reactively, use standard Rust `for` loops inside the `view!`:

```rust
let fruits = vec!["Apple", "Banana", "Cherry"];

view! {
    <ol>
        {
            for fruit in fruits {
                <li>{ fruit }</li>
            }
        }
    </ol>
}
```
