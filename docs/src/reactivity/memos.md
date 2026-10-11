# Derived State with Memos

Memos are cached computed signals that only re-evaluate when their tracked dependencies change.

---

## 1. Creating a Memo

Use `create_memo` or the terse factory `memo`:

```rust
use veloasm::prelude::*;

let count = signal!(2);

// Derived state: recalculates only when `count` updates.
// (`RwSignal` is `Copy`, so it moves straight into the memo closure.)
let doubled = memo!(move || count.get() * 2);

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

let full_name = memo!(move || format!("{} {}", first_name.get(), last_name.get()));
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

---

## 4. Ownership: `Memo` Is `Copy`

Like the signal handles, `Memo<T>` is `Copy` — move it into any number of
`move ||` closures (reactive attributes, effects, event handlers) with no
`.clone()` calls:

```rust
let count = signal!(0);
let is_even = memo!(move || count.get() % 2 == 0);

// The same memo moves into two reactive closures — no clones needed.
view! {
    <div>
        <span style:display={ move || if is_even.get() { "inline" } else { "none" } }>
            "even"
        </span>
        <span style:display={ move || if is_even.get() { "none" } else { "inline" } }>
            "odd"
        </span>
    </div>
}
```

The recomputation effect is owned by the enclosing scope when the memo is
created inside one (torn down with its owner), and retained for the app
lifetime otherwise — the same rule as [`effect!`](effects.md).

