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

### Children form

Instead of `label`, nest any child nodes to style richer link content:

```rust
view! {
    <Link to="/blog/hello">
        <span class="emoji">"✨"</span>
        "Hello post"
    </Link>
}
```

### Active state

Pass `active_class` to get a class applied automatically when the browser is on (or inside) the
link's route. Matching is boundary-safe: `/blog` activates for `/blog` and `/blog/:slug`, but
never `/blogxyz`; `/` activates only on the exact root.

```rust
view! {
    <Link to={ paths::BLOG } label="Blog" active_class="is-active" />
}
```

---

## 2. The `<Head>` Component (document title & meta)

`<Head>` sets the browser `document.title` (and optional `<meta name>` tags) whenever the node
renders. Because the router re-renders the matched page on navigation, a `<Head>` placed in
each page keeps the title/meta in sync client-side — the SPA analogue of Next.js metadata.

```rust
#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="Blog" meta={ vec![("description".to_string(), "My blog".to_string())] } />
            <h1>"Blog"</h1>
        </div>
    }
}
```

```rust
#[page]
pub fn post() -> DomNode {
    let slug = FRouter::param("slug").unwrap_or_default();
    view! {
        <article>
            <Head title={ format!("Post: {slug}") } />
            <h1>{ slug }</h1>
        </article>
    }
}
```

Notes:
- `title` accepts a string literal or (via braces) a reactive `String`/`format!`.
- `meta` is a `Vec<(String, String)>` of `(name, content)` pairs.
- `<Head>` renders nothing into `<body>`. Place it **inside** the page's single root element.
- A navigation to a page without a given meta tag removes the previous page's tags, so metas
  never stack.

---

## 3. Programmatic Navigation (`navigate_to`)

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
