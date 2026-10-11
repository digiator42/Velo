# Side Effects & Tracking

Effects run arbitrary side effects whenever their tracked reactive dependencies change.

---

## 1. Creating an Effect

Use `create_effect(closure)`:

```rust
use veloasm::prelude::*;

let theme = signal!("dark".to_string());

let theme_reader = theme.clone();
let handle = create_effect(move || {
    let current_theme = theme_reader.get();
    web_sys::console::log_1(&format!("Theme changed to: {}", current_theme).into());
});
```

* **Runs Immediately**: The effect executes once synchronously upon creation to track dependencies.
* **Auto-Subscribes**: Any `.get()` called during this run adds the signal to the effect's dependency set.
* **Auto-Re-runs**: Whenever any subscribed signal notifies changes, the effect re-runs.

---

## 2. Effect Lifetimes: Handles & Scopes

`create_effect` returns an `EffectHandle`. **Dropping the last handle disposes
the effect** — cleanup runs, and future signal changes no longer trigger it:

```rust
let handle = create_effect(move || { /* ... */ });
drop(handle); // disposed
```

Handles are `Clone`; whichever clone drops last wins, so you can pass them
around freely.

`effect!(...)` is the shorthand with owner-aware defaults:

```rust
// App setup (no scope open): retained for the app lifetime.
effect!(move || analytics.track(route.get()));

// Inside a #[component] / keyed row / veloasm::scope: adopted by the owner and
// disposed when that owner's node is torn down.
effect!(move || render_log(tail.get()));
```

See [Effect Lifecycle & Resource Cleanup](../dom-and-lifecycle/effect-cleanup.md)
for the ownership model in detail.

---

## 3. Cleanup Closures with `create_effect_with_cleanup`

When setting up browser timers, web socket subscriptions, or external event listeners inside an effect, use `create_effect_with_cleanup`:

```rust
let is_active = signal!(true);

let is_act = is_active.clone();
let handle = create_effect_with_cleanup(
    move || {
        if is_act.get() {
            web_sys::console::log_1(&"Subscribed to service".into());
        }
    },
    || {
        web_sys::console::log_1(&"Unsubscribed from service".into());
    }
);
```

The cleanup callback runs automatically when the effect is disposed (or when the owning `DomNode` is unmounted).

