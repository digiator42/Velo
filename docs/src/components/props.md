# Component Props & Reactivity

Components can accept static arguments, reactive signals, or shared models as properties. Props are
**named**: `#[component]` generates a `<Name>Props` struct, and `view!` matches JSX attribute keys to
the corresponding field names (in any order). The generated `NameProps` type must be in scope at the
call site.

---

## 1. Passing Reactive Signal Props

To keep a child component reactive to parent state, pass a `ReadSignal<T>`, `RwSignal<T>`, or `Signal<T>` handle as a prop:

```rust
use velo::prelude::*;

#[component]
fn MetricDisplay(title: String, value: RwSignal<i32>) {
    view! {
        <div class="metric-card">
            <h4>{ title }</h4>
            <div class="value">{ value }</div>
        </div>
    }
}

// In the parent view:
let score = signal!(100);

view! {
    <MetricDisplay title="Player Score".into() value={ score } />
}
```

---

## 2. Plain / Owned Props

For values that do not change over the lifetime of the component, pass plain Rust types (`String`, `bool`, `u32`, etc.):

```rust
#[component]
fn StatusBadge(label: String, is_primary: bool) {
    view! {
        <span class:primary={ is_primary } class="badge">
            { label }
        </span>
    }
}
```
