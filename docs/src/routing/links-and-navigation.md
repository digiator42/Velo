# Links & Client-Side Navigation

---

## 1. The `<Link>` Component

Use `<Link to="..." label="..." />` to create hyperlinks that navigate via the HTML5 History API without triggering a full page reload:

```rust
use velo::prelude::*;

view! {
    <nav class="navbar">
        <Link to="/" label="Home" />
        <Link to="/tasks" label="Tasks" />
        <Link to="/settings" label="Settings" />
    </nav>
}
```

---

## 2. Programmatic Navigation (`navigate_to`)

To navigate programmatically (e.g. after a form submission or button click), call `velo::navigate_to`:

```rust
use velo::navigate_to;

view! {
    <button on:click={ move |_| {
        // Perform actions...
        navigate_to("/dashboard");
    }}>
        "Go to Dashboard"
    </button>
}
```

This immediately updates the browser URL bar, pushes a history state, and triggers the router to mount the target page component.
