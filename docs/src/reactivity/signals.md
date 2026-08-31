# Signals (Split Read & Write)

Signals are the primary atomic unit of reactive state in Velo.

---

## 1. Creating a Split Signal

Use `create_signal(initial_value)` to create a reactive value. It returns a tuple containing a read handle (`ReadSignal<T>`) and a write handle (`WriteSignal<T>`):

```rust
use velo::prelude::*;

let (count, set_count) = create_signal(0);
```

---

## 2. Reading Signal Values (`.get()`)

Calling `.get()` returns a clone of the current value and registers the caller as a dependency if called inside an active effect or memo computation:

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
set_count.set(10);
```

### Mutating In-Place with `.update()`
For complex structs or collections where you want to mutate the existing value in-place:

```rust
let (user, set_user) = create_signal(User { name: "Alice".into(), age: 30 });

set_user.update(|u| {
    u.age += 1;
});
```

---

## 4. Ownership & Clones

Both `ReadSignal<T>` and `WriteSignal<T>` implement `Clone`. Cloning a signal handle is cheap — it only clones an internal `Rc` reference to the shared signal cell:

```rust
let count_for_button = count.clone();
let set_count_for_button = set_count.clone();

let on_click = move |_| {
    set_count_for_button.set(count_for_button.get() + 1);
};
```
