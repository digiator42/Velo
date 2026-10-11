# Reactive Collections (`SignalVec`)

`SignalVec<T>` is a reactive vector that allows mutating collections with fine-grained notifications.

---

## 1. Creating a `SignalVec`

```rust
use veloasm::prelude::*;

#[derive(Clone)]
struct TodoItem {
    id: u32,
    title: String,
    done: bool,
}

// Terse helper
let items = signal_vec(vec![
    TodoItem { id: 1, title: "Buy groceries".into(), done: false },
    TodoItem { id: 2, title: "Write Rust code".into(), done: true },
]);
```

---

## 2. Collection Methods

`SignalVec` provides standard vector mutation methods that notify subscribers:

```rust
// Push a new item
items.push(TodoItem { id: 3, title: "Ship veloasm app".into(), done: false });

// Remove item at index
let removed = items.remove(0);

// Get length (tracks reactivity in effects/memos)
let count = items.len();

// Clear the vector
items.clear();
```

---

## 3. In-Place Batch Updates (`.with_mut()`)

When making multiple modifications to a collection simultaneously, use `.with_mut()` to perform in-place mutation with a single subscriber notification:

```rust
items.with_mut(|vec| {
    for item in vec.iter_mut() {
        item.done = true;
    }
});
```

---

## 4. Bounded Collections (capped helpers)

Two helpers keep a collection at a fixed maximum size with a **single**
notification, no matter how many items were evicted:

```rust
// Rolling window: push newest, evict from the FRONT past the cap.
// (e.g. keep only the last 200 log lines)
logs.push_capped(log_line, 200);

// Newest-first log: insert at the FRONT, evict from the BACK past the cap.
// (e.g. a chat transcript where old messages fall off the end)
messages.insert_front_capped(message, 500);
```

Both are the right tool for high-frequency streams (logs, chat, tickers)
where unbounded growth would otherwise leak memory and slow reconciliation.

---

## 5. Keyed Rendering

Combine `SignalVec` with veloasm's keyed `for` loop in the `view!` macro to reconcile DOM nodes by stable key:

```rust
view! {
    <ul>
        {
            for item in items key = |it: &TodoItem| it.id {
                <li>{ item.title.clone() }</li>
            }
        }
    </ul>
}
```
Inserting or removing items only updates the affected DOM nodes without tearing down existing elements.

