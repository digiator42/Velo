# Velo

High-performance, client-side, fine-grained reactive SPA framework for Rust → WebAssembly.
No Virtual DOM — state changes update the real DOM surgically.

## Design goals

- **Tiny & fast**: size-tuned `wasm` binary (see `[profile.release]` in `Cargo.toml`).
- **Fine-grained reactivity**: update exactly the text node / attribute that changed.
- **Light & client-side**: runs entirely in the browser. No server-side rendering.

## Workspace layout

- `crates/velo` — the unified single package: reactivity engine (`Signal`, split `ReadSignal`/`WriteSignal`, `create_signal`, `create_effect`, `create_memo`), `DomNode` wrapper over `web-sys`, the `view!` macro support and `RenderDynamic`, and the client-side router (`Router`, `Route`, `Link`, `FRouter`). Everything in one `prelude`.
- `crates/macro` — the companion proc-macro package (`view!`, `#[component]`, `routes!`).
- `examples/` — `counter-spa`, `server-monitor-dashboard`, `features-demo`, `todo-app`, `memo-unwrap-check`, `arrow-closures`.

## Quickstart

```rust
use velo::prelude::*;

fn app() -> DomNode {
    // Split a signal into a read handle and a write handle.
    let (count, set_count) = create_signal(0);

    view! {
        <div>
            <h2>{ count }</h2>            {/* signal auto-unwraps: no `.get()` */}
            <button on:click={ move |_| set_count.set(count.get() + 1) }>
                "Increment"
            </button>
        </div>
    }
}
```

Build & serve with [Trunk](https://trunkrs.dev):

```sh
trunk serve --open examples/counter-spa/index.html
```

## Reactivity model

### Signals (read / write split)

```rust
let (count, set_count) = create_signal(0);   // (ReadSignal<i32>, WriteSignal<i32>)
count.get();                                  // read
set_count.set(5);                             // write
set_count.update(|c| *c += 1);               // mutate in place
```

For single-handle ergonomics, `Signal::new(v)` still exists (`Signal<T>` is `Clone`,
with `.get()` / `.set()` / `.update()`, and `.split()` / `.read_only()` / `.write_only()`).

### Derived state — `create_memo`

```rust
let (base, set_base) = create_signal(2);
let doubled = create_memo({
    let base = base.clone();
    move || base.get() * 2
});
// `doubled` auto-unwraps in `view! { { doubled } }` (no `.get()` needed in the view).
// In Rust code use `.get()` for the raw value (recomputes only when `base` changes).
```

### Effects

`create_effect(|| { ... })` runs immediately and re-runs whenever any signal it reads changes.

## The `view!` macro

- **Auto-unwrap**: `{ count }` and `name={ user_name }` automatically subscribe to and unwrap signals.
  Write handles / plain values work unchanged.
- **Components**: `view! { <UserCard name={ user_name.clone() } role="Admin" /> }` — uppercase tags
  are components. Pass a `ReadSignal<T>` prop to keep the child reactive.
- **Events**: `<button on:click={ move |e| ... }>` binds a closure (the attribute prefix `on:` is special).
- **Reactive attributes**: `class={ theme }`, `disabled={ is_busy }` update live.
- **Lists**: `{ for item in items { view! { <li>{ item }</li> } } }`.

## Router

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
Read params with `FRouter::param("id")`.

## Status

Core reactivity, DOM rendering, the `view!` macro, and the router are functional and
compile to `wasm32-unknown-unknown`. The framework is intentionally minimal — see the
plan under `.hermes/plans/` for the roadmap (components ergonomics, reactive `class:`/`style:`
toggles, forms, keyed lists, stores/context, and a CLI).

## What's new

### `#[component]` macro

Write components as plain functions — the macro sets the return type to `DomNode`,
so the body can end in a `view!` tail expression.

```rust
#[velo::component]
fn UserCard(name: String, active: bool) {
    view! {
        <li class:active={ active } class="card">
            <span>"User: " { name }</span>
        </li>
    }
}
```

### Reactive `class:` / `style:` toggles

```rust
let (dark, set_dark) = create_signal(false);

view! {
    <div class:dark={ dark }>                       // toggles the `dark` class
        <button on:click={ move |_| set_dark.set(!dark.get()) }>"Toggle"</button>
        <p style:color={ color_signal }>"Reactive color"</p>   // reactive inline style
    </div>
}
```

### Keyed reactive lists (`SignalVec` + `<For key = ...>`)

`SignalVec<T>` is a reactive collection; the keyed `for` loop reconciles the real DOM
by stable key instead of re-rendering the whole list.

```rust
let users = SignalVec::new(vec![User { id: 1, name: "Ada".into() }]);

view! {
    <ul>
        {
            for u in users key = |u: &User| u.id {
                <UserCard name={ u.name.clone() } active={ u.id % 2 == 0 } />
            }
        }
    </ul>
}
```

Add/remove items and the framework inserts/removes only the changed nodes.

### Stores / context

Share state across a component tree without prop drilling:

```rust
#[derive(Clone)]
struct Theme { dark: bool }

provide_context(Theme { dark: false });   // in the root

#[velo::component]
fn ThemeBadge() {
    let theme = use_context::<Theme>();     // None if no ancestor provided it
    view! { <div>{ theme.map(|t| t.dark) }</div> }
}
```

`with_context(value, || { ... })` scopes a value to a subtree and restores it afterwards.

## Quickstart

```rust
use velo::prelude::*;

fn app() -> DomNode {
    // Split a signal into a read handle and a write handle.
    let (count, set_count) = create_signal(0);

    view! {
        <div>
            <h2>{ count }</h2>            {/* signal auto-unwraps: no `.get()` */}
            <button on:click={ move |_| set_count.set(count.get() + 1) }>
                "Increment"
            </button>
        </div>
    }
}
```

Build & serve with [Trunk](https://trunkrs.dev):

```sh
trunk serve --open examples/features-demo/index.html
```

See `examples/features-demo` for a runnable showcase of all four features.
