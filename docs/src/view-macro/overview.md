# The `view!` Macro Overview

The `view!` procedural macro allows you to write HTML-like markup in Rust that compiles directly into high-performance DOM node creation calls.

---

## 1. Basic Syntax

```rust
use velo::prelude::*;

fn render_card() -> DomNode {
    view! {
        <div class="card">
            <h2>"Card Title"</h2>
            <p>"This is a static paragraph."</p>
        </div>
    }
}
```

---

## 2. Syntax Rules

1. **Tag Names**:
   * Lowercase tags (e.g. `<div>`, `<button>`, `<span>`) create standard HTML DOM elements.
   * Uppercase tags (e.g. `<UserCard>`, `<HeaderNav>`) invoke component functions.
2. **Text Literals**: Static text must be enclosed in double quotes: `"Hello World"`.
3. **Rust Expressions**: Wrap any dynamic Rust value, signal, closure, or conditional block in `{ ... }`.
4. **Self-Closing Tags**: Elements without children can self-close: `<input type="text" />`, `<hr />`, `<br />`.
5. **Attributes**: Passed with `attr="static"` or `attr={ dynamic_expr }`.
