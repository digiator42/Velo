# Velo

High-performance, client-side, fine-grained reactive SPA framework for Rust → WebAssembly.
No Virtual DOM — state changes update the real DOM surgically.

## Design goals

- **Tiny & fast**: size-tuned `wasm` binary (see `[profile.release]` in `Cargo.toml`).
- **Fine-grained reactivity**: update exactly the text node / attribute that changed.
- **Light & client-side**: runs entirely in the browser. No server-side rendering.

## Quickstart

```sh
velo new my-app
cd examples/my-app
velo dev
# open http://localhost:8080
```

Or build & serve any example with [Trunk](https://trunkrs.dev) directly:

```sh
trunk serve --open examples/counter-spa/index.html
```

## Quick example

```rust
use velo::prelude::*;

fn app() -> DomNode {
    let count = signal!(0);

    view! {
        <div>
            <h2>{ count }</h2>   {/* signal auto-unwraps: no `.get()` */}
            <button on:click={ move |_| count.update(|c| *c += 1) }>
                "Increment"
            </button>
        </div>
    }
}
```

## Reactivity model

### Signals — `signal!`

```rust
let count = signal!(0);          // RwSignal<i32> (get/set/update, no clone needed)
count.get();                      // read
count.set(5);                     // write
count.update(|c| *c += 1);        // mutate in place
```

For a split read/write pair:

```rust
let (count, set_count) = create_signal(0);   // (ReadSignal<i32>, WriteSignal<i32>)
```

### Derived state — `memo!`

```rust
let doubled = memo!(move || count.get() * 2);
// `doubled` auto-unwraps in `view! { { doubled } }` (no `.get()` needed in the view).
```

### Effects — `effect!`

```rust
effect!(move || log(count.get()));
// With cleanup:
effect!(
    move || attach_listener(),
    move || detach_listener(),
);
```

### Reactive lists — `signal_vec!`

```rust
let users = signal_vec![User { id: 1, name: "Ada".into() }];
users.push(User { id: 2, name: "Bob".into() });
```

## The `view!` macro

### Auto-unwrap

`{ count }` and `name={ user_name }` automatically subscribe to and unwrap signals.
Write handles / plain values work unchanged.

### JS-style arrow closures

```rust
// Sync handler
<button on:click={ move |_| count.update(|c| *c += 1) }>"+"</button>

// Shorthand arrow (no `move ||` boilerplate)
<button on:click={ () => count.update(|c| *c += 1) }>"+"</button>

// With event param
<input on:input={ (e) => input.set(e.target().value()) } />

// Async handler — `.await` directly
<button on:click={ async () => {
    let data = fetch_json::<User>("/api/user/1").await.unwrap();
    user.set(data);
}}>"Load"</button>
```

### Components

Uppercase tags are components. Pass a `ReadSignal<T>` prop to keep the child reactive.

```rust
#[velo::component]
fn UserCard(name: String, active: bool) {
    view! {
        <li class:active={ active } class="card">
            <span>\"User: \" { name }</span>
        </li>
    }
}

// Usage
view! { <UserCard name={ user_name.clone() } role=\"Admin\" /> }
```

### Reactive attributes

```rust
view! {
    <div class={ theme } disabled={ is_busy }>
        <p style:color={ color_signal }>"Reactive color"</p>
    </div>
}
```

### Reactive class toggles

```rust
view! {
    <div class:dark={ is_dark } class:active={ is_active } class="base">
        "Toggles `dark` and `active` classes reactively, keeping the `base` class."
    </div>
}
```

### Conditional class lists — `class_names!`

```rust
view! {
    <div class={ class_names!(
        "card",
        if selected.get() { "card--selected" } else { "card--dim" },
        (n > 0).then_some("card--has-items"),
    )}>
        "Conditional classes without clobbering the base."
    </div>
}
```

### Two-way form binding

```rust
view! {
    <input bind:value={ input } />
    <input type="checkbox" bind:checked={ is_done } />
}
```

`bind:value` drives the **live DOM property** (`HtmlInputElement.set_value`), not just the attribute — so IME, default values, and programmatic resets behave correctly.

### `on:submit` sugar

```rust
view! {
    <form on:submit={ () => {
        // event.prevent_default() is called automatically
        save();
    }}>
        <input bind:value={ input } />
        <button type="submit">"Save"</button>
    </form>
}
```

### Keyed reactive lists

```rust
view! {
    <ul>
        { for u in users key = |u: &User| u.id {
            <UserCard name={ u.name.clone() } active={ u.id % 2 == 0 } />
        } }
    </ul>
}
```

Add/remove items and the framework inserts/removes only the changed nodes.

### Control flow — `<Show>` / `<Suspense>`

```rust
view! {
    <Show when={ move || !list.is_empty() } fallback={ DomNode::text(\"\") }>
        <p>"Has items"</p>
    </Suspense>
}

view! {
    <Suspense loading={ resource.loading() } fallback={ view! { <p>\"Loading…\"</p> } }>
        { move || match resource.value() {
            Some(data) => view! { <DataView data={ data } /> },
            None => view! { <p>\"Loading…\"</p> },
        } }
    </Suspense>
}
```

### Error boundaries

```rust
view! {
    <ErrorBoundary fallback={ view! { <p>\"Something went wrong.\"</p> } }>
        <RiskyComponent />
    </ErrorBoundary>
}
```

Inside the subtree, call `boundary_fault("message")` to trigger the fallback (wasm-safe; works with `panic = "abort"`).

## Async data — `create_resource`

```rust
let resource = create_resource(|| async move {
    fetch_json::<Vec<User>>("/api/users").await
});

view! {
    <Suspense loading={ resource.loading() } fallback={ view! { <p>\"Loading…\"</p> } }>
        { move || match resource.value() {
            Some(Ok(users)) => view! { <UserList users={ users } /> },
            Some(Err(_)) => view! { <p>\"Failed to load.\"</p> },
            None => view! { <p>\"Loading…\"</p> },
        } }
    </Suspense>
}
```

## Stores / context

```rust
#[derive(Clone)]
struct Theme { dark: false }

provide!(Theme { dark: false });   // in the root

#[velo::component]
fn ThemeBadge() {
    let theme: Option<Theme> = context!();
    view! { <div>{ theme.map(|t| t.dark) }</div> }
}
```

`with_context(value, || { ... })` scopes a value to a subtree and restores it afterwards.

## Routing

### File-based routing (`velo::app!`)

The recommended approach. Drop files in `src/app/` and the macro generates the route table at compile time:

```
src/app/
  layout.rs            # root layout (wraps every route)
  page.rs              # "/"
  blog/page.rs         # "/blog"
  blog/[slug]/page.rs  # "/blog/:slug" (typed param)
  blog/loading.rs      # per-route loading fallback
  blog/error.rs        # per-route error boundary
  not-found.rs         # 404
```

```rust
use velo::prelude::*;
velo::app!();

#[wasm_bindgen(start)]
pub fn main() {
    let shell = view! {
        <div id="app-container">
            <nav>
                <Link to={ paths::INDEX } label="Home" />
                <Link to={ paths::blog_slug("hello") } label="Hello post" />
            </nav>
            <main>
                <Router routes={ velo_app::routes() } />
            </main>
        </div>
    };
    mount(shell);
}
```

`<Link to>` is validated at compile time — a bad path fails to build.

### Programmatic routing

```rust
let routes = vec![
    Route { path: "/", component: home_page },
    Route { path: "/dashboard", component: dashboard_page },
    Route { path: "/u/:id", component: user_page },
    Route { path: "/**", component: not_found },
];
view! { <Router routes={ routes } /> }
```

`<Link to="/dashboard" label="Dashboard" />` navigates without a full reload.
Read params with `FRouter::param::<T>("id")` or `use_param::<T>("id")`.

### `<Link>` prefetch

```rust
<Link to="/users" label="Users" prefetch />
```

Hovering (or tab-focusing) the link fires an early `fetch` of the destination. The navigation reuses the same in-flight request — only one network request.

### `<Head>` metadata

```rust
view! {
    <Head title="My App" meta={ vec![("description".to_string(), "...".to_string())] } />
}
```

Sets `document.title` + `<meta>` tags on each navigation. Stale tags from the previous route are removed.

## Workspace layout

- `crates/velo` — the unified runtime: reactivity, DOM, router, fetch, error overlay.
- `crates/macro` — proc macros: `view!`, `#[component]`, `routes!`, `#[route]`, `app!`, `route_path!`.
- `crates/velo-cli` — `velo new` / `velo dev` / `velo build` (wraps Trunk).
- `examples/` — runnable apps (counter-spa, blog, async-dashboard, features-demo, typed-nav, dynamic, …).

## Status

All roadmap pillars (5.P0–5.P11) are complete: zero-clone reactivity, file-based routing, Suspense/error boundaries, typed navigation, dev error overlay, `class_names!`, `use_dynamic`, and the `velo` CLI. The framework compiles to `wasm32-unknown-unknown`.
