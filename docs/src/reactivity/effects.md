# Side Effects & Tracking

Effects run arbitrary side effects whenever their tracked reactive dependencies change.

---

## 1. Creating an Effect

Use `create_effect(closure)`:

```rust
use velo::prelude::*;

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

## 2. Cleanup Closures with `create_effect_with_cleanup`

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
