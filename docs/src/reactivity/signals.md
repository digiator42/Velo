# Signals

Signals are the primary atomic unit of reactive state in veloasm.

---

## 1. Creating a Signal with `signal!`

Use `signal!(initial_value)` to create a reactive value with a single combined
read+write handle (`RwSignal<T>`):

```rust
use veloasm::prelude::*;

let count = signal!(0);
```

`signal!` is shorthand for `signal(value)`, which wraps `RwSignal<T>`. One handle
reads *and* writes, and the handle is `Copy`, so you move it into any number
of closures with no `.clone()` boilerplate.

> `create_signal(value)` still exists and returns the older split `(ReadSignal,
> WriteSignal)` tuple pair. Prefer `signal!` for new code — the single `RwSignal`
> is more ergonomic and removes the need to clone two handles. See
> [RwSignal & Ergonomic Handles](rw-signals.md).

---

## 2. Reading Signal Values (`.get()`)

Calling `.get()` returns a clone of the current value and registers the caller
as a dependency if called inside an active effect or memo computation:

```rust
let current_val = count.get();
```

Inside the `view!` macro, `.get()` is **called automatically**:

```rust
view! {
    <p>"Count is: " { count }</p> // Auto-unwrapped!
}
```

---

## 3. Writing Signal Values (`.set()` & `.update()`)

### Replacing the Value with `.set()`
```rust
count.set(10);
```

### Mutating In-Place with `.update()`
For complex structs or collections where you want to mutate the existing value
in-place:

```rust
let user = signal!(User { name: "Alice".into(), age: 30 });

user.update(|u| {
    u.age += 1;
});
```

---

## 4. Ownership & Copies

`RwSignal<T>` is `Copy` — a thin shared handle over the signal cell. Move it
freely into `move` closures; every copy reads and writes the same value:

```rust
let on_click = move |_| {
    count.set(count.get() + 1);
};
```

The same holds for `ReadSignal<T>`, `WriteSignal<T>`, and `Memo<T>`. Only
`SignalVec<T>` (and your own data) still needs `.clone()` per closure.

