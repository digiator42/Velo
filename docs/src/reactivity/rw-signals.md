# `RwSignal` & Ergonomic Handles

When managing local state that is both read and written in multiple closures, splitting handles into `(ReadSignal, WriteSignal)` can lead to variable bookkeeping. veloasm provides `RwSignal<T>` and the `signal!` macro for zero-boilerplate state.

---

## 1. Using `signal!` / `RwSignal<T>`

`signal!(initial_value)` returns an `RwSignal<T>` combining both read and write capabilities in a single handle:

```rust
use veloasm::prelude::*;

let count = signal!(0);

// Reading
let val = count.get();

// Writing
count.set(5);
count.update(|c| *c += 1);
```

---

## 2. In Closures and Event Handlers

The single `RwSignal` handle is `Copy`, so it moves into any number of
closures with no cloning at all:

```rust
let count = signal!(0);

view! {
    <div>
        <p>"Count: " { count }</p>
        <button on:click={ move |_| count.set(count.get() + 1) }>
            "Increment"
        </button>
    </div>
}
```

---

## 3. Two-Way Form Binding with `RwSignal`

`RwSignal` seamlessly connects to veloasm's two-way form bindings:

```rust
let query = signal!(String::new());

view! {
    <input type="text" placeholder="Search..." bind:value={ query } />
}
```

When the user types into the `<input>`, `query` updates automatically; when `query.set(...)` is called in code, the `<input>` element immediately reflects the new value.

