# Signals

Signals are the primary atomic unit of reactive state in Velo.

---

## 1. Creating a Signal with `signal!`

Use `signal!(initial_value)` to create a reactive value with a single combined
read+write handle (`RwSignal<T>`):

```rust
use velo::prelude::*;

let count = signal!(0);
```

`signal!` is shorthand for `signal(value)`, which wraps `RwSignal<T>`. One handle
reads *and* writes, so you clone it once and move it into closures.

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

## 4. Ownership & Clones

`RwSignal<T>` implements `Clone`. Cloning a signal handle is cheap — it only
clones an internal `Rc` reference to the shared signal cell:

```rust
let count_for_button = count.clone();

let on_click = move |_| {
    count_for_button.set(count_for_button.get() + 1);
};
```
