# Class & Style Toggles

Velo provides special attribute prefixes (`class:` and `style:`) for fine-grained styling and class list manipulations without string concatenation.

---

## 1. Reactive Class Toggling (`class:name`)

Use `class:<classname>={ bool_expression }` to add or remove individual CSS classes based on reactive boolean state:

```rust
let is_active = signal(true);
let is_dark_mode = signal(false);

view! {
    <div
        class="base-card"
        class:active={ is_active }
        class:dark={ is_dark_mode }
    >
        <p>"Card content"</p>
    </div>
}
```

* When `is_active.get()` is `true`, `"active"` is added via `element.classList.add("active")`.
* When `is_active.get()` becomes `false`, `"active"` is removed via `element.classList.remove("active")`.
* Existing static classes (like `"base-card"`) remain untouched.

---

## 2. Reactive Inline Styles (`style:property`)

Use `style:<property>={ string_expression }` to bind CSS properties directly:

```rust
let primary_color = signal("hsl(210, 100%, 50%)".to_string());
let font_size = signal("1.25rem".to_string());

view! {
    <div
        style:color={ primary_color }
        style:font-size={ font_size }
    >
        "Dynamic styled text"
    </div>
}
```

Multiple `style:` bindings on the same element are merged automatically without clobbering sibling CSS properties.

---

## 3. Conditional Class Lists (`class_names!`)

For building a whole `class` string from a mix of unconditional and conditional
classes, use the `class_names!` join helper. It skips `None` values and empty
strings and joins the rest with spaces — ideal for `class={ class_names!(..) }`:

```rust
let selected = signal(false);
let items = signal(0);

view! {
    <div class={ class_names!(
        "card",                                  // always on
        "card--selected",                        // plain conditional (string)
        (items.get() > 5).then_some("card--busy"), // Option<&str>: None is skipped
        (!selected.get()).then_some("card--idle"),
    )}>
        "Card content with a computed class list"
    </div>
}
```

* Plain `&str`/`String` args and `Option<&str>`/`Option<String>` args may be mixed.
* `None` (e.g. from `bool.then_some(..)`) and empty strings contribute nothing.
* The result is a single `String` you can also reuse outside `view!`:
  `let cls = class_names!("a", x.then_some("b"));`

> Tip: prefer `class:name={ signal }` for a single on/off toggle driven by one
> boolean, and `class_names!` when several conditions combine into one class list.
