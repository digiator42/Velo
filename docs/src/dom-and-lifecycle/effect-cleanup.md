# Effect Lifecycle & Resource Cleanup

In client-side applications, failing to unbind event listeners, drop intervals, or close websockets when components unmount causes memory leaks. veloasm handles resource cleanup through its node ownership tree: **everything a node creates is owned by that node and dies with it.**

---

## 1. Node-Owned Resources

Every `DomNode` wraps a shared resource record (`Rc<RefCell<NodeRes>>`):

```
┌──────────────────────────────────────────────────────────┐
│                        DomNode                           │
│  ├─ raw_node: web_sys::Node                              │
│  └─ res: Rc<RefCell<NodeRes>>                            │
│       ├─ node: web_sys::Node        (for listener removal)│
│       ├─ children: Vec<...>         (absorbed subtrees)  │
│       ├─ effects: Vec<EffectHandle> (bindings/reconcilers)│
│       └─ listeners: Vec<(event, Closure)>                │
└──────────────────────────────────────────────────────────┘
```

When the last handle to a node's resource drops:

1. Every stored listener runs `remove_event_listener` (closures released).
2. Every `EffectHandle` drops — disposing its effect (see §2).
3. Absorbed child resources release bottom-up, so the whole subtree cleans up.

---

## 2. `EffectHandle` — Last-Owner Disposal

Every reactive binding (`reactive_text`, `reactive_attribute`, `toggle_class`, `render_expression`, keyed lists, `#[component]` bodies) produces `EffectHandle`s that are retained by their node instead of being `mem::forget`ed; `on(...)` listeners are stored alongside them as `(event, Closure)` pairs.

Handles are `Clone`, and **whichever clone drops last disposes the effect**:

```rust
let h = create_effect(move || { /* ... */ });
let h2 = h.clone();
drop(h);   // still alive — h2 exists
drop(h2);  // disposed: cleanup runs, future signal changes skip it
```

Disposal is leak-free by construction: signals store `Weak` references to subscribers, so a disposed effect is skipped and pruned on the next notification.

---

## 3. Absorption & Release (`append` / `release`)

`parent.append(&child)` inserts the child's DOM node **and absorbs** its resource record, so the subtree stays reactive even after your local `child` handle drops:

```rust
parent.append(&child);   // parent now owns child's effects/listeners
```

When a child leaves its parent, its resource is **released** first so its effects are disposed instead of lingering:

```rust
parent.release(&child);           // manual removal
// the keyed reconciler and dynamic-expression swaps call this automatically
```

---

## 4. Effect Scopes — `effect!` Knows Its Owner

`effect!(|| ...)` behaves differently depending on where it runs:

* **Outside any scope** (app setup code): the handle is retained — the effect lives for the app lifetime, so a bare `effect!(...)` statement stays reactive.
* **Inside a scope** (component body, keyed-list row, `veloasm::scope`): the effect is adopted by the enclosing scope and **disposed when that owner is torn down**.

Scopes come from three places:

```rust
// #[component] bodies are wrapped automatically:
#[component]
fn Counter() {
    effect!(move || log(count.get())); // dies with the component's node
}

// Keyed rows get a per-row scope: row effects die with the row.
{ for item in items key = |i| i.id { <Row item={item} /> } }

// Manual scopes for arbitrary code:
let (_value, guard) = veloasm::scope(|| {
    effect!(move || poll(status.get()));
});
// `guard` dropped => effect disposed
```

`veloasm::scoped_node(|| ...)` is the underlying primitive `#[component]` and the router use: effects created while building a node attach to that node.

**Router pages are scoped per navigation** — effects created while rendering a route are attached to the page node and disposed on the next navigation, so long-lived SPAs don't accumulate page effects.

> **Gotcha — don't create `Rc` cycles:** an effect stored on a node must not capture a `DomNode` clone of that *same* node (the closure would own the resource that owns the closure). Capture `web_sys` raw nodes instead, or capture `Rc::downgrade` handles.

---

## 5. Manual Cleanup Registration

To register custom cleanup logic on an effect:

```rust
use veloasm::create_effect_with_cleanup;

let handle = create_effect_with_cleanup(
    || {
        // Setup logic (e.g. subscribe to web socket)
    },
    || {
        // Cleanup logic (runs exactly once, when the effect is disposed)
    }
);
```

Pair it with the timer handles for interval teardown:

```rust
let mut tick = interval(1_000, move || ticks.update(|t| *t += 1));
create_effect_with_cleanup(
    move || { let _ = ticks.get(); },
    move || tick.cancel(),   // interval stops with the effect
);
```

Dropping the last `EffectHandle` yourself is also valid manual lifecycle control:

```rust
let h = create_effect(/* ... */);
// ...
drop(h); // disposed immediately
```

