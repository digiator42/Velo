# Context API (`provide_context` & `use_context`)

The Context API allows you to share global or subtree-scoped state across deeply nested components without passing props manually through every intermediate layer (prop drilling).

---

## 1. Defining a Context Struct

Any type that implements `Clone + 'static` can be stored in context:

```rust
#[derive(Clone)]
pub struct ThemeContext {
    pub is_dark: RwSignal<bool>,
    pub accent_color: String,
}
```

---

## 2. Providing Context (`provide_context`)

Call `provide_context(value)` at the root or parent level:

```rust
use velo::prelude::*;

pub fn run_app() {
    // Seed global context
    provide_context(ThemeContext {
        is_dark: signal(false),
        accent_color: "hsl(210, 100%, 50%)".into(),
    });

    mount_to_id("app", app_shell());
}
```

---

## 3. Consuming Context (`use_context`)

In any descendant component, call `use_context::<T>()`. It returns an `Option<T>` containing a clone of the provided value:

```rust
#[component]
fn ThemeToggle() {
    let theme = use_context::<ThemeContext>()
        .expect("ThemeContext must be provided in root");

    let is_dark_for_click = theme.is_dark.clone();

    view! {
        <button on:click={ move |_| {
            is_dark_for_click.update(|dark| *dark = !*dark);
        }}>
            "Toggle Theme"
        </button>
    }
}
```

---

## 4. Scoping Context with `with_context`

To override a context value for a specific subtree and restore the previous value afterward:

```rust
let sub_tree = with_context(
    ThemeContext { is_dark: signal(true), accent_color: "red".into() },
    || {
        // Child components constructed inside this closure receive the overridden theme
        render_subview()
    }
);
```
