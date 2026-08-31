# Elements, Text & Auto-Unwrapping

---

## 1. Static Text vs. Dynamic Expressions

### Static Text
Static text is written as quoted string literals:

```rust
view! {
    <h1>"Welcome to Velo"</h1>
}
```

### Dynamic Text
Dynamic values and expressions are wrapped in braces `{ ... }`:

```rust
let user_name = "Ada Lovelace";
let age = 36;

view! {
    <div>
        <p>"User: " { user_name }</p>
        <p>"Age: " { age }</p>
    </div>
}
```

---

## 2. Automatic Signal Unwrapping

When a signal or memo is placed inside `{ ... }`, the `view!` macro automatically tracks and unwraps its value:

```rust
let count = signal(42);

view! {
    <div>
        <span>{ count }</span> // Automatically renders "42" and updates on signal change!
    </div>
}
```

You do **not** need to write `{ move || count.get() }`. The `view!` macro handles unwrapping and reactive subscription automatically.

---

## 3. Supported Render Types

Any type implementing the `RenderDynamic` trait can be rendered directly inside `{ ... }`:
* `&str`, `String`
* Integer primitives (`i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64`, `usize`)
* Floating-point primitives (`f32`, `f64`)
* `bool`
* `DomNode`
* `Vec<DomNode>`
