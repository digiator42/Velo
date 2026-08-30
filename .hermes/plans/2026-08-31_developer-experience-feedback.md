# Velo Developer-Experience Feedback (from building `examples/todo-app`)

> **For Hermes:** Read this before/while implementing the improvement plan
> (`2026-08-30_velo-improvement-plan.md`). This is hands-on feedback from actually writing
> a real multi-page app against the current API, plus the exact compile failures hit and
> the current blocker that must be fixed first.

**Date:** 2026-08-31
**Scope:** A new routed "Task Manager" example (`examples/todo-app/`) exercising the full
framework — routing, `SignalVec` keyed lists, `create_memo` derived state, context
(`provide_context`/`use_context`), the `#[component]` macro, and an `<input>` form.
**Constraint honored:** No Velo crate was modified. Only `examples/` + `Cargo.toml` members.

---

## 1. API regressions hit while writing the example

### 1.1 `create_memo` return type changed → broke documented auto-unwrap
- README documents `create_memo(...) -> ReadSignal<T>` and `view! { { memo } }` auto-unwrapping.
- Under the current WIP, `create_memo` returns a new `Memo<T>` wrapper.
- `Memo<T>` did **not** implement `ViewValue` (the auto-unwrap trait) at first, so
  `view! { { total } }` failed with `E0277 Memo<u32>: ViewValue / PlainViewValue / RenderDynamic`.
- "Fix" in current tree: `dom` now has `impl ViewValue for velo_core::Memo<T>` (`dom/src/lib.rs:89`).
- **Action:** Commit the `ViewValue for Memo` impl, update the README return type, and add a test
  that `view! { { memo } }` compiles. The docs/API mismatch is a real footgun.

### 1.2 Read/write signal split — two easy `E0599`s
`create_signal` returns `(ReadSignal, WriteSignal)`. Event handlers often need BOTH handles, and
only `WriteSignal` has `.set()` while only `ReadSignal` has `.get()`. I hit:
- `no method named 'set' on ReadSignal` (cloned the read handle for the write closure)
- `no method named 'get' on WriteSignal` (vice-versa)
Errors are clear, but the split forces extra `clone()`s and handle bookkeeping in every controller.

### 1.3 Keyed list render closure requires `Copy` (`dom/src/lib.rs:353`)
`render_signal_vec<T,K,FKey,FRender>` requires `FRender: Fn(&T) -> DomNode + Copy`. Because the
per-item render closure must be `Copy`, it **cannot capture any non-`Copy` value**. I first wrote
`<TaskRow state={ app_state.clone() } task={ t.clone() } />` inside a keyed `for` and got
`E0277 AppState: Copy not satisfied`. Workaround: have `TaskRow` read `AppState` from context
instead of capturing it. This is an ergonomic trap — components in keyed lists can't take state
props unless those props are `Copy`.

---

## 2. Feature gaps that most hurt building a real app

### 2.1 No form input binding (biggest missing piece)
Reading an `<input>` value means hand-writing web-sys plumbing in the handler:
```rust
on:input={ move |e: web_sys::Event| {
    if let Some(t) = e.target() { if let Ok(el) = t.dyn_into::<HtmlInputElement>() { ... } }
} }
```
There is no `bind:value` / `v-model` equivalent. `value={ signal }` is just a reactive attribute.
This is the first wall any non-toy app hits. (Roadmap already lists forms — bump priority.)

### 2.2 No fine-grained mutation for list items
To toggle a task's `done` flag I had to read the whole `SignalVec`, rebuild a new `Vec`, and
`with_mut(|v| *v = updated)` — which re-renders the whole list. There's no per-item write handle
or field-level update. Works, but defeats the "update exactly the node that changed" pitch for
the most common list interaction.

### 2.3 Toggling a class with a static bool is fine, but `class:`/`style:` read signals only via `signal_value!`
`class:done={ task.done }` (a plain `bool`) compiled, but I could not rely on the memo auto-unwrap
(see 1.1) so had to push `.get()` manually.

---

## 3. What worked well (keep doing this)

- **`view!` macro**: components (`<TaskRow/>`), keyed `for ... key = |t| t.id`, `on:`, `class:`,
  string/expr text — all ergonomic and compose nicely.
- **`#[component]`**: writing `fn TaskRow(task: Task)` with a `view!` tail expression is clean.
- **Context**: `provide_context(AppState{...})` + `use_context::<AppState>()` made cross-page shared
  state trivial and also sidestepped the 1.3 `Copy` trap cleanly.
- **Router**: `Route { path, component }`, `Router`, `Link` worked as documented; good catch-all `/**`.

---

## 4. Recommended priority order for Hermes

1. **P0 — Restore a clean, committed, Force-built-green workspace** (Section 0). Nothing else is
   testable until `crates/dom` compiles fresh. Add a CI `cargo build --workspace --target wasm32-unknown-unknown` gate.
2. **P1 — Land + document the `Memo` API** (1.1): commit `ViewValue for Memo`, fix README, add a compile test.
3. **P1 — Form input binding** (2.1): `bind:value`/`bind:checked` in the macro + dom layer.
4. **P2 — Relax keyed-list `Copy` bound** (1.3): allow the render closure to capture owned values
   (e.g. require `Clone` instead of `Copy`, or record per-row captures).
5. **P2 — Per-item/reactive field update for `SignalVec`** (2.2), or at least document the
   whole-rebuild pattern.
6. **P3 — Reduce split-signal cloning** (1.2): consider a combined handle or helper so controllers
   don't juggle two handles + clones.

---

## 5. Verification recipe (use this, not `cargo build --workspace` alone)

```sh
# Must force recompile so stale caches can't mask errors:
cargo build -p velo_core --target wasm32-unknown-unknown
cargo build -p velo_dom --target wasm32-unknown-unknown
# Then the example, both debug and release:
cargo build -p todo-app --target wasm32-unknown-unknown
cargo build -p todo-app --release --target wasm32-unknown-unknown
# Full pipeline:
trunk build examples/todo-app/index.html
```
Only treat a feature as "done" when a fresh `-p` build (debug AND release) is green.

---

> **NEW DIRECTIVE (2026-08-31):** Shift focus to **developer luxury** — make Velo as
> effortless as **Next.js**. Sections 6+ below extend the earlier "make it work" feedback into a
> "make it delightful" roadmap. Each `L#` item: what Next.js gives us → how to replicate in Velo →
> files → verify. Target the client-side experience first (SSR stays optional).

---

## 6. Developer Luxury — Next.js parity roadmap

**North star:** a developer should go from "idea" to "routing app with typed data, forms and
live-reload DX" without thinking about Reactivity/borrows/macros — exactly the calm Next.js gives
with `app/` folders, `useState/useMemo/useContext`, `<Link>`, `bind`, and `create-next-app`.

### L1. Zero-clone, auto-tracked reactivity (the #1 luxury)
**Next.js:** `const [count, setCount] = useState(0)` — no `.get()`, no `.set()`, state just appears.
**Velo today:** `create_signal` → `(ReadSignal, WriteSignal)`; logs at runtime/compile require
`.get()`/`.set()` + `clone()` juggling (see §1.2).
**Goal:** `let count = signal(0); view! { <h2>{ count }</h2> }` and `count.set(count.get()+1)`.
- Auto-unwrap EVERY `{ signal }` and `attr={ signal }` via `ViewValue` (already mostly there —
  make it cover `Memo` too, §1.1).
- Add an ergonomic combined handle `RwSignal<T>` (Leptos-style): `.get()`, `.set()`, `.update()`,
  `.clone()`, Display + Deref to `T` so `format_args!` and pass-through work without touches.
- `signal()`, `signal_vec()`, `memo()`, `effect()` as terse factory names in `prelude`.
- Auto-span `move` so handlers capture by clone under the hood (macro emits `move || ...` + clones).
- **Files:** `crates/core` (RwSignal), `crates/dom` (ViewValue coverage), `crates/macro` (auto-move).
- **Verify:** rewrite `examples/todo-app` and `examples/counter-spa` with zero explicit `.get()`/`.clone()`
  in the views.

### L2. File-style routing + `#[route]` pages (app-router feel)
**Next.js:** `app/dashboard/page.tsx` = a route, zero manual route tables, typed `params`.
**Velo today:** manual `let routes = vec![Route { path, component }]` in `run_app`.
**Goal:**
```rust
#[route("/dashboard/:id")]
fn dashboard() -> DomNode { let id = use_param::<u64>("id"); view!{ ... } }
```
- `#[route(path)]` attribute registers the page into a global route registry (const-friendly).
- Or a single declarative module: `routes! { "/" => home, "/dashboard/:id" => dashboard, "/**" => not_found }`
  — one place, but purely declarative (no magic filesystem at wasm runtime).
- Typed params: `use_param::<T>(key)` + derive `FromStr`; `use_query()`; `use_route()` for current path.
- `<Link to="/dashboard/${id}">children</Link>` with children support (already have `LinkChildren`).
- **Files:** `crates/router` (registry + typed param/query hooks), `crates/macro` (`#[route]`, `routes!`).
- **Verify:** `examples/todo-app` pages defined via `#[route]` with zero manual table; typed `use_param`.

### L3. Component composition: `children` + props structs (JSX feel)
**Next.js:** `<Layout>{children}</Layout>`; props as a typed object; arbitrary nesting.
**Velo today:** components take positional typed args; no children-first ergonomics; list `Copy` trap (§1.3).
**Goal:**
```rust
#[component]
fn Badge(color: String, chip: bool) { view!{ <span class:chip={chip}>{ ? }</span> } }
#[component]
fn Panel(children: Vec<DomNode>, title: String) -> DomNode { view!{ <section><h3>{title}</h3>{children}</section> } }
```
- Macro: when a component has a `children` param, pass `<Panel ...>{ ... }</Panel>` children into it
  (already pushes a `view!{}` last arg — extend so the component *receives* it as a named `children`).
- Support a props-struct form: `#[component(Props)]` where `Props` is a derive struct → call `.field`.
- Fix the `Copy` bound (§1.3) so components in lists can take owned props without the context hack.
- **Files:** `crates/macro` (component children + props-struct), `crates/dom` (list bound).
- **Verify:** `examples/features-demo` `UserCard` + a new nested `Panel>{children}</Panel>`.

### L4. Form binding sugar (the biggest wall — §2.1, elevate to luxury)
**Next.js:** `value={x} onChange={(e)=>set(e.target.value)}` / controlled inputs, or `useForm`.
**Goal:** two-way without web-sys plumbing:
```rust
let (name, _) = signal(String::new());
view!{ <input bind:value={ name } /> }          // input → set(name), signal → value
view!{ <input type="checkbox" bind:checked={ done } /> }
bool bind:checked on checkbox
```
- `bind:` prefix in macro: on input event, read `el.value()`/`checked`, call `.set()`.
- Reactive `value`/`checked` attribute stays two-way.
- Add `<form on:submit>` with `event.prevent_default()` sugar.
- **Files:** `crates/macro` (`bind:` attr), `crates/dom` (bind helper on DomNode).
- **Verify:** `examples/todo-app` add-task form rewritten to pure `bind:value`, zero event handlers.

### L5. Async data + `<Show>`/`<Suspense>` + error boundary (loading/streaming parity)
**Next.js:** `async function` server components, `loading.tsx`, `error.tsx`, suspense fallbacks.
**Goal (client-first):**
```rust
let data = create_resource(|| async { fetch_json(url).await });  // signal of Result<T,Err>
view!{ <Show when={ data.loading() } fallback={|| view!{<p>"Loading…"</p>} }>{ data }</Show> }
```
- `create_resource` in `crates/core` (async producer + reactive `loading`/value/error).
- `<Show when={cond}>` true/false child; `<Suspense>` fallback child.
- `<ErrorBoundary>` that catches panics / renders fallback on component subtree.
- **Files:** `crates/core` (`create_resource`), `crates/macro` (`<Show>`, `<Suspense>`, `<ErrorBoundary>`), `crates/dom`.
- **Verify:** new `examples/async-dashboard` with a delayed fetch shows fallback then real data.

### L6. Dev loop = `create-next-app` + HMR + error overlay
**Next.js:** `npx create-next-app` scaffold + instant refresh + friendly error overlay in-browser.
**Goal:**
- `velo new <project>` (Task 5.1 real CLI) scaffolds a Trunk-ready package + `index.html` + first page.
- In-dev **error overlay**: a script that injects a styled panel showing the current Rust compile
  error + `file:line` on top of the page (not just a plain Trunk failure).
- HMR: document/automate `trunk serve --watch`; keep component state across reload where possible.
- **Files:** `crates/cli` (real), `examples/*/index.html` (dev overlay hook), README.
- **Verify:** `velo new demo && cd demo && velo dev` → page up, then introduce a compile error → overlay shows it.

### L7. Styling parity
**Next.js:** `className`, CSS modules, Tailwind first-class, CSS-in-JS.
**Goal (pragmatic):**
- Ensure `class="a b"`, `class:{cond}`, `style:{prop}` all compose on the same element (verify
  `toggle_class`/`reactive_style` merge — see §2.3).
- Support `style:{prop}={ signal }` WITHOUT clobbering sibling style props (already merged — keep).
- Document Tailwind via `<style>` in `index.html` + Trunk `data-trunk` CSS assets (fits the size ethos).
- Add `class_names!`-style join helper for conditional class lists.
- **Files:** `crates/dom` (verify merge), `crates/macro` (`class_names!`), README + example styling.
- **Verify:** `examples/todo-app` dark mode + card `class:{active}` compose with static classes.

### L8. Typed navigation & routes (typedRoutes parity)
**Next.js:** `typedRoutes` makes invalid `<Link href>` a compile error.
**Goal:** `<Link to=...>` and `navigate_to(...)` validated at compile time against registered
`#[route]` paths, with `:param` placeholders checked; `use_param` returns the typed scalar.
- **Files:** `crates/router` + `crates/macro` (a `route_path!` macro that type-checks against routes!).
- **Verify:** a bad `to=` fails to compile with a clear message.

### L9. Cross-cutting: keep the size ethos
Every luxury above must preserve the "tiny wasm, fine-grained, no VDOM" identity. Prefer
macros/compile-time work over runtime overhead; keep `[profile.release]` (opt-z + LTO) intact and
add a `wasm-size` CI step (Task 5.3) so DX additions don't silently bloat.

---

## 7. Luxury priority order (do after P0 workspace fix + §4 P1s)

1. **L1** zero-clone reactivity — the single highest-leverage luxury; unlocks every other comfort.
2. **L4** form binding — closes §2.1, the biggest wall for real apps.
3. **L2** `#[route]` + typed params — replaces manual route tables with Next-style pages.
4. **L3** children + props structs + `Copy` fix — natural component authoring.
5. **L6** `velo new` + error overlay — the onboarding "aha".
6. **L5** async resources + `<Show>` — real-world data UIs.
7. **L7** styling + **L8** typed routes — polish.

**Demo milestones (each independently runnable as an example):**
- `counter-spa` rewritten with L1 → zero `.get()`/`.clone()`.
- `todo-app` with L1+L4+L2 → `#[route]` pages + `bind:value` form.
- new `async-dashboard` (L5) and `layout-children` (L3) examples.

---

## 8. Mounting API redesign (kills the "extra wrapper div" + magic string)

> **Problem raised:** a next-generation SPA shouldn't force the developer into
> `mount_to_id("app", root)` — a stringly-typed `id` that `.expect()`s (panics only at runtime if
> misspelled) and an unconditional **append inside** the container, producing an unavoidable
> wrapper: `<div id="app"><div class="page">…</div></div>`. No way to render multiple root
> siblings, no returned handle to unmount, no way to mount into a plain node / body / shadow root.
> Next.js/React/Svelte root apps through a **node** with a clean fragment root — Velo should too.

### 8.1 Current behavior (verify before changing)
`crates/dom/src/lib.rs:408` `mount_to_id(id, root)`:
- looks up `get_element_by_id(id)` (panics if absent),
- `container.append_child(root.raw_node)` → root becomes a **child** of `#app` (wrapper div).

`DomNode` already has `fragment()` (`lib.rs:127`) which the browser auto-unpacks — the missing piece
is a root API that uses it so nothing wraps the app.

### 8.2 Recommended API (professional, layered)
Provide ONE ergonomic entry plus lower-level handles; keep the old function as a thin shim
(deprecated) to avoid breaking the existing examples.

```rust
// Convenience: no magic string required; default to <body> with a fragment root (no wrapper).
pub fn mount(root: DomNode) -> RootHandle {
    mount_at(&document().body().expect("no body"), root)
}

// Explicit mount target as a real node, NOT a string. Replacement, not append.
pub fn mount_at(target: &web_sys::Node, root: DomNode) -> RootHandle { ... }

// string form kept for migration, now annotated (appends and returns a handle)
#[deprecated(note = "prefer mount()/mount_at(); see 8.3")]
pub fn mount_to_id(id: &str, root: DomNode) -> RootHandle { ... }
```

**`RootHandle`** (returned, not forgotten) owns the root + its effect handles and exposes:
- `.unmount()` — remove the root node(s) from the DOM and dispose every root-level effect,
  closing the current "append and forget" leak gap against `DomNode`'s own Drop semantics,
- `Drop for RootHandle` → auto-unmount, so a parent scope can tear the whole app down.

**Rules that make it professional:**
1. **Mount target is a `web_sys::Node`/`DomNode`, never a raw `&str`** — one less runtime panic
   class. Add `mount_into(&DomNode)` when aiming at a node you already built.
2. **Replace by default, append as an explicit option** (`mount_at_replace` vs `mount_at_append`),
   so the container is owned by the app rather than doubled.
3. **Fragment root by default** → root siblings expand directly into the target, zero wrapper.
   Accept any `DomNode` (a single element still works) so users pick their structure.
4. **Return `RootHandle`** always — enables unmount, remount, hot-reload teardown, and SSR
   hydration later (attach effects to existing nodes by `data-velo-id`).
5. **Keep `document()` accessible** for those who want a custom root; never reach into `body` by
   string.

### 8.3 Migration for existing examples
`examples/counter-spa`, `server-monitor-dashboard`, `features-demo`, `todo-app` all call
`mount_to_id("app", ...)`. Update the root expressions to use fragments (already fine) and swap to
`mount(root)` (body) OR keep `#app` but call `mount_at(&#app_node, root)` with replace semantics so
there is no wrapper. Update README Quickstart accordingly.

### 8.4 Verify
- `examples/todo-app`: `document.body` should contain the app's nodes **directly** (`<nav>`,
  `<div class="page">`) — assert no extra wrapper via a test / DevTools.
- `let handle = mount(...); handle.unmount();` removes everything and disposes effects.
- No string-literal mount point panics remain in any example.


