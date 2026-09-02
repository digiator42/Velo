# Derived State with Memos

Memos are cached computed signals that only re-evaluate when their tracked dependencies change.

---

## 1. Creating a Memo

Use `create_memo` or the terse factory `memo`:

```rust
use velo::prelude::*;

let count = signal!(2);

// Derived state: recalculates only when `count` updates
let doubled = memo({
    let count = count.clone();
    move || count.get() * 2
});

assert_eq!(doubled.get(), 4);

count.set(5);
assert_eq!(doubled.get(), 10);
```

---

## 2. Automatic Dependency Tracking

Memos can read any number of signals or other memos. They automatically subscribe to all signals read during their execution:

```rust
let first_name = signal!("Ada".to_string());
let last_name = signal!("Lovelace".to_string());

let full_name = memo({
    let first = first_name.clone();
    let last = last_name.clone();
    move || format!("{} {}", first.get(), last.get())
});
```

---

## 3. Template Usage & Auto-Unwrapping

Memos implement `ViewValue`, so they auto-unwrap directly in `view!` templates:

```rust
view! {
    <div>
        <h2>{ full_name }</h2> // Automatically tracks and updates!
    </div>
}
```
