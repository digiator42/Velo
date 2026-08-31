# Update Batching

When updating multiple signals in succession, notifications can trigger multiple redundant effect runs. Velo provides `batch()` to group updates into a single notification cycle.

---

## 1. The `batch()` Function

```rust
use velo::prelude::*;

let (first_name, set_first) = create_signal("Ada".to_string());
let (last_name, set_last) = create_signal("Lovelace".to_string());

let full_name = memo({
    let first = first_name.clone();
    let last = last_name.clone();
    move || format!("{} {}", first.get(), last.get())
});

// Without batch(): `full_name` recomputes twice (once after set_first, once after set_last)
// With batch(): `full_name` recomputes ONCE after both sets complete
batch(|| {
    set_first.set("Grace".to_string());
    set_last.set("Hopper".to_string());
});
```

---

## 2. Nested Batches

`batch()` handles nesting safely. Only the outermost `batch()` flushes pending notifications:

```rust
batch(|| {
    set_a.set(1);
    batch(|| {
        set_b.set(2);
    });
    set_c.set(3);
}); // Single synchronized notification cycle runs here
```
