# Reactive Attributes

Element attributes in Velo can be static string constants or live reactive expressions.

---

## 1. Static Attributes

```rust
view! {
    <a href="/dashboard" target="_blank" rel="noopener">"Dashboard"</a>
}
```

---

## 2. Dynamic & Reactive Attributes

Passing a signal or dynamic value to an attribute creates a live subscription:

```rust
let avatar_url = signal("https://example.com/avatar.png".to_string());
let user_id = signal(12345);

view! {
    <img src={ avatar_url } id={ user_id } alt="User Avatar" />
}
```

Whenever `avatar_url` or `user_id` updates, the corresponding DOM attribute updates immediately.

---

## 3. Boolean Attributes (`disabled`, `checked`, `readonly`)

Boolean attributes are automatically toggled based on the truthiness of the expression:

```rust
let is_submitting = signal(false);

view! {
    <button disabled={ is_submitting }>
        "Submit Form"
    </button>
}
```

* When `is_submitting` is `true`, `disabled=""` is set on the DOM element.
* When `is_submitting` is `false`, the `disabled` attribute is removed completely.
