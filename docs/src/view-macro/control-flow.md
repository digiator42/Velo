# Conditional Rendering & Control Flow

In Velo, dynamic branches are written using standard Rust `if / else` expressions and `match` statements inside `{ ... }`.

---

## 1. Simple Conditions

Pass a closure returning a `DomNode` to render elements conditionally. Closures are detected automatically via AST analysis — no extra wrapping is needed:

```rust
let is_logged_in = signal(false);

let auth = is_logged_in.clone();
view! {
    <div>
        {
            move || {
                if auth.get() {
                    view! { <p>"Welcome back, user!"</p> }
                } else {
                    view! { <button>"Log in"</button> }
                }
            }
        }
    </div>
}
```

---

## 2. Empty Fallbacks

To conditionally render an element or nothing at all, return an empty text node `DomNode::text("")`:

```rust
let show_details = signal(false);

let details = show_details.clone();
view! {
    <div>
        {
            move || {
                if details.get() {
                    view! { <div class="details">"Secret details"</div> }
                } else {
                    DomNode::text("")
                }
            }
        }
    </div>
}
```

---

## 3. Pattern Matching (`match`)

Use `match` expressions to switch between states:

```rust
#[derive(Clone)]
enum Status {
    Loading,
    Success(String),
    Error(String),
}

let status = signal(Status::Loading);

let s = status.clone();
view! {
    <div class="status-panel">
        {
            move || match s.get() {
                Status::Loading => view! { <span>"Loading..."</span> },
                Status::Success(data) => view! { <span class="ok">{ data }</span> },
                Status::Error(err) => view! { <span class="err">{ err }</span> },
            }
        }
    </div>
}
```
