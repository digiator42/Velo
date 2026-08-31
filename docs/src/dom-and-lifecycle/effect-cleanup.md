# Effect Lifecycle & Resource Cleanup

In client-side applications, failing to unbind event listeners, drop intervals, or close websockets when components unmount causes memory leaks. Velo handles resource cleanup through its node ownership tree.

---

## 1. Automatic Effect Teardown with `EffectHandle`

Every reactive binding (`reactive_text`, `reactive_attribute`, `toggle_class`, `on`, `render_expression`) produces an `EffectHandle`. 

`DomNode` stores a vector of active `EffectHandle` instances:
```
┌────────────────────────────────────────────────────────┐
│                        DomNode                         │
│  ├─ raw_node: web_sys::Node                            │
│  └─ effects: Vec<EffectHandle>                         │
│         ├─ Class toggler effect handle                 │
│         ├─ Text update effect handle                   │
│         └─ Event listener cleanup handle               │
└────────────────────────────────────────────────────────┘
```

When a `DomNode` is removed from the DOM and dropped, its `effects` are dropped. Dropping an `EffectHandle` disposes the effect and removes it from all signal subscriber registries.

---

## 2. Parent-Child Effect Absorption (`append`)

When you append a child node to a parent (`parent.append(&child)`), the parent **absorbs** the child's effect handles. The entire subtree's lifetime is bound to the parent element, ensuring that removing a parent component cleans up all descendant listeners automatically.

---

## 3. Manual Cleanup Registration

To register custom cleanup logic on an effect:

```rust
use velo_core::create_effect_with_cleanup;

let handle = create_effect_with_cleanup(
    || {
        // Setup logic (e.g. subscribe to web socket)
    },
    || {
        // Cleanup logic (runs when effect is disposed)
    }
);
```
