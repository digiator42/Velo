# Route Parameters & Query Strings

---

## 1. Dynamic Route Parameters (`:param`)

With `app!` file routing, a dynamic param is a `[name]` folder under `src/app/`.
The macro derives the route for you — no route table to maintain:

```text
src/app/
└── users/
    ├── page.rs          # "/users"
    └── [id]/page.rs     # "/users/:id"
```

```rust
// src/app/users/[id]/page.rs
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    let user_id = FRouter::use_param::<String>("id").unwrap_or_else(|| "unknown".into());
    view! {
        <div class="user-page">
            <h2>"User Profile for ID: " { user_id }</h2>
        </div>
    }
}
```

The macro also emits a typed `paths::users_id(id)` builder in `velo_app::paths`,
so you can link to the route with a runtime value:

```rust
use velo::prelude::*;
// assumes: velo::app!() in src/lib.rs and src/app/users/[id]/page.rs
<Link to={ velo_app::paths::users_id("42") } label="User 42" />
```

(In `src/lib.rs` after `use velo::prelude::*`, `velo_app` is in scope, so this
is usually written `paths::users_id("42")`.)

For a manual `routes!`/`Vec<Route>` table the same `:id` param applies:

```rust
let routes = routes! {
    "/" => home_page,
    "/users/:id" => user_profile_page,   // manual form of /users/[id]/page.rs
};
```

### Reading Parameters in Page Components:
Use `FRouter::use_param::<T>(key)` with automatic type parsing:

```rust
fn cluster_node_page() -> DomNode {
    let cluster_id = FRouter::use_param::<u32>("cluster_id").unwrap_or(0);
    let node_id   = FRouter::use_param::<u32>("node_id").unwrap_or(0);

    view! {
        <div class="node-page">
            <h2>"Cluster " { cluster_id } " / Node " { node_id }</h2>
        </div>
    }
}
```

Any type implementing `FromStr` is supported (`String`, `u32`, `i64`, etc.). Returns `None` if the parameter is missing or fails to parse.

### Untyped Parameters (Legacy)

For raw string access, use `FRouter::params()`:

```rust
let user_id = FRouter::params().get("id").cloned().unwrap_or_else(|| "unknown".to_string());
```

---

## 2. Reading Query Strings (`?search=foo&page=2`)

Use `FRouter::use_query::<T>(key)` to extract and parse URL query parameters:

```rust
fn search_page() -> DomNode {
    let query = FRouter::use_query::<String>("q").unwrap_or_default();
    let page  = FRouter::use_query::<u32>("page").unwrap_or(1);

    view! {
        <div>
            <h2>"Search results for: " { query }</h2>
            <p>"Current page: " { page }</p>
        </div>
    }
}
```

Query parameters are URL-decoded automatically.

### Raw Query Access

For the full query map, use `FRouter::query()`:

```rust
let all_params = FRouter::query();
let sort = all_params.get("sort").cloned().unwrap_or_else(|| "relevance".into());
```

---

## 3. Getting the Current Path

Use `FRouter::use_route()` to get the current URL path string:

```rust
fn breadcrumb() -> DomNode {
    let path = FRouter::use_route();
    view! {
        <nav class="breadcrumb">
            <span>"Current route: " { path }</span>
        </nav>
    }
}
```
