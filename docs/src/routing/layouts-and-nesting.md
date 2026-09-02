# Layouts & Nesting

A **layout** is a `#[layout]` function that wraps a set of routes, providing a
persistent shell (header, nav, footer). In Velo the layout **shell is mounted
once** and survives navigation — only the routed leaf inside it swaps.

---

## 1. Defining a layout

Any directory under `src/app/` can have a `layout.rs`. Its function takes a
single `child: DomNode` argument (the routed page) and returns the wrapped
`DomNode`:

```rust
// src/app/layout.rs — root layout wrapping every page
use velo::prelude::*;

#[layout]
pub fn layout(child: DomNode) -> DomNode {
    view! {
        <div class="app-shell">
            <header>
                <span class="brand">"My App"</span>
                <nav>
                    <Link to={ paths::INDEX } label="Home" />
                    <Link to={ paths::POSTS } label="Posts" />
                </nav>
            </header>
            <main>
                { child }
            </main>
        </div>
    }
}
```

---

## 2. Layouts nest by segment

A `layout.rs` in a sub-directory only wraps the routes under that segment:

```text
src/app/
├── layout.rs           # root layout — wraps everything
├── page.rs             # "/"
├── about/page.rs       # "/about"
└── posts/
    ├── layout.rs       # wraps only /posts and /posts/:slug
    ├── page.rs         # "/posts"
    └── [slug]/page.rs  # "/posts/:slug"
```

`app!` composes the chain from the outermost segment to the matched leaf. A page
under `/posts` renders through the `posts` layout **and** the root layout; `/`
and `/about` only go through the root layout.

---

## 3. Shell persistence

Because the shell is mounted once and only the leaf outlet changes, state
declared in a layout **survives navigation** between sibling routes:

```rust
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let clicks = signal!(0);   // created once when the shell first mounts
    view! {
        <div class="app-shell">
            <header>
                <button on:click={ move |_| clicks.set(clicks.get() + 1) }>
                    "shell count: " { clicks }
                </button>
            </header>
            <main>{ child }</main>
        </div>
    }
}
```

Navigate between `/posts` and `/posts/hello-velo` — the shell (and `clicks`) is
**not** re-created. The router swaps only the leaf inside `<main>`, leaving the
layout subtree and its local signals intact.

---

## 4. Layout-scoped providers

Use layout functions to `provide_context` values that every page under that
segment reads via `use_context`. Since the layout persists, so does the context
across navigations:

```rust
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    provide_context(theme_signal);
    // ... wrap child
}
```

---

## See also

- [`examples/layout-children`](../../examples) — a root layout whose `clicks`
  counter survives navigation, with a keyed route leaf.
- [File-based routing](../routing/router-and-routes.md) — the `app!` conventions
  for `layout.rs`, `loading.rs`, `error.rs`, and `not-found.rs`.