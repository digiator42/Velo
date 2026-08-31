# Reactivity Overview & Mental Model

Velo's reactivity model is inspired by SolidJS and fine-grained reactive engines. It operates on a **push-pull dependency graph** with automatic tracking.

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│     Signal      │ ────► │      Memo       │ ────► │     Effect      │
│  (Source State) │       │ (Derived State) │       │ (DOM Mutator)   │
└─────────────────┘       └─────────────────┘       └─────────────────┘
```

---

## 1. How Automatic Tracking Works

When an effect runs:
1. Velo sets a thread-local identifier pointing to the currently executing effect (`ACTIVE_EFFECT_ID`).
2. Any signal whose `.get()` method is invoked reads this thread-local and adds the active effect to its internal subscriber list (`subscribers`).
3. When the effect finishes executing, `ACTIVE_EFFECT_ID` is cleared.
4. Future calls to `.set()` on that signal notify all registered subscribers, re-executing the effect.

---

## 2. No Component Re-renders

In Velo, **component functions run exactly once**.

```rust
#[component]
fn Counter() {
    let (count, set_count) = create_signal(0);

    // This print statement only outputs ONCE when the component is mounted!
    web_sys::console::log_1(&"Component mounted".into());

    view! {
        <div>
            <span>{ count }</span>
            <button on:click={ move |_| set_count.set(count.get() + 1) }>+</button>
        </div>
    }
}
```

When `set_count.set(...)` is called, `Counter()` is **not** called again. Only the specific reactive text node binding `{ count }` re-evaluates its value and writes to the browser DOM.
