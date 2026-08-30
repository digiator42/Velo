# Velo Framework Improvement Plan (Developer Experience, Features, Syntax)

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Turn Velo from a promising fine-grained-reactive prototype into a framework developers
actually enjoy building SPAs with: fewer `clone()`s, reactive HTML attributes, clean component props,
SSR/SSR-streaming, typed signals, a working CLI, and real docs — without breaking its size-tuned WASM core.

**Architecture:** Velo is a client-side, fine-grained-reactive Rust→WASM SPA framework (no Virtual DOM).
Reactivity core lives in `crates/core` (`Signal` + `create_effect` + a thread-local effect registry),
the DOM layer in `crates/dom` (`DomNode`), the `view!` macro in `crates/macro`, routing in `crates/router`,
and a unified facade + `prelude` in `crates/velo`. The biggest DX wins come from (1) making reactivity
invisible in the macro and (2) making attributes/components/typed-state first-class. A thin server
crate + CLI give SSR and tooling parity with Leptos/Yew.

**Tech Stack:** Rust 2021, wasm-bindgen, web-sys, syn/quote/proc-macro2, Trunk (dev server/build).
Optional: `axum` for SSR server, `tracing` for diagnostics.

**Current snapshot (verified 2026-08-30):** `cargo check -p velo_core` passes; `cli` crate is an
EMPTY manifest (0 bytes) with no `src/` — it is dead weight in the workspace and must be fixed or removed.
The `view!` macro, `velo_dom`, `velo_router` compile for `wasm32-unknown-unknown`.

---

## 0. Foundational fixes (do first — everything depends on a healthy workspace)

### Task 0.1: Repair or remove the broken `cli` crate
- **Problem:** `crates/cli/Cargo.toml` is 0 bytes and `crates/cli/src/` does not exist. `cargo build` on
  the workspace will fail once the cli is actually depended on / built.
- **Fix (chosen):** Either delete `crates/cli` and its workspace entry, OR scaffold a real CLI here
  (see Task 9). Recommended: scaffold it as the real CLI in Task 9 and fill it in now.
- **Files:** `crates/cli/Cargo.toml` (write real manifest), `crates/cli/src/main.rs`, `Cargo.toml` (workspace members).
- **Verify:** `cargo build --workspace --target wasm32-unknown-unknown 2>&1 | tail` exits 0.

### Task 0.2: Add a shared `prelude` re-export of `Signal::new` ergonomics & turn on `deny(warnings)` later
- Add `pub use velo_core::Signal;` already present; ensure `use velo::prelude::*;` covers everything used in examples.
- **Verify:** `cargo build -p counter-spa --target wasm32-unknown-unknown` (after adding trunk-proof build) succeeds.

### Task 0.3: CI + local build script
- **Create:** `xtask` or a `Makefile`/`justfile` with `check`, `test`, `build-examples`, `fmt`, `clippy`.
- **Files:** `Makefile`, `.github/workflows/ci.yml`.
- **Verify:** `make check` green.

---

## 1. Developer Experience — make reactivity invisible (highest impact)

### Task 1.1: Auto-track signals in the `view!` macro (kill manual `.get()`/`.clone()`)
- **Objective:** `view! { <h2>{ count }</h2> }` and `view! { <div class={ count }> }` should
  automatically subscribe to `count` inside an effect — no `count.get()` / `count.clone()` noise.
- **Approach:** In `crates/macro/src/lib.rs`, when an expression resolves to a `Signal<_>` (detect via
  type or, simpler, blanket: wrap any `ReactiveExpression` body in a closure and *also* emit a `.get()`
  for `Signal` types), emit `velo_dom::DomNode::render_expression(move || #expr)` and, for the text
  case, call `.get()` automatically when the type is a `Signal`.
- **Design note:** The cleanest path is a `Signal` method `fn render(self) -> impl FnMut()->String`
  plus macro detection of `Signal`. Start with: if the brace expr is a single identifier bound to a
  `Signal`, emit `move || expr.get()`. This removes ~80% of boilerplate in examples.
- **Files:** `crates/macro/src/lib.rs` (ReactiveExpression arm), `crates/core/src/lib.rs` (add `Signal::render` or `.to_string_signal()`).
- **Test:** example `counter-spa` rewritten with `view! { <h2>{ count }</h2> }`.
- **Verify:** rebuild example; counter increments on click.

### Task 1.2: `#[velo::component]` attribute macro — ergonomic components & props
- **Objective:** Replace hand-written `fn UserCard(name: Signal<String>, role: String) -> DomNode`
  with a component that (a) accepts typed props struct or individual reactive props, (b) lets the body
  use `view!` directly, (c) hides the `-> DomNode` + `Signal` plumbing.
- **Approach:** Add `crates/macro` `#[component]` that wraps `fn` into a `fn(...) -> DomNode` and
  allows `view!` inside without `return`. Also support prop structs:
  `#[component] fn UserCard(name: ReadSignal<String>, role: String) -> impl View { view!{ ... } }`.
  (Pairs with Task 2.1 typed signals.)
- **Files:** `crates/macro/src/lib.rs` (new `#[proc_macro_attribute] component`), `crates/velo/src/lib.rs` re-export.
- **Verify:** `examples/counter-spa/src/components.rs` rewritten using `#[component]`.

### Task 1.3: Reactive `class` / `style` / `disabled` literals & class toggling
- **Objective:** `view! { <div class:active={ is_on } class="btn" style:color={ color }> }`.
- **Approach:** Extend `VAttr` parsing: `class:name={expr}` → `DomNode::toggle_class(name, move||expr)`;
  `style:key={expr}` → `DomNode::reactive_style(key, move||expr)`. Keep `on:` for events.
- **Files:** `crates/macro/src/lib.rs`, `crates/dom/src/lib.rs` (add `toggle_class`, `reactive_style`, `set_class`).
- **Verify:** example with a toggle button that adds/removes a class.

### Task 1.4: Reactive `if` / `Show` and keyed `<For>` in the macro
- **Objective:** `view! { { if show.get() { view!{<p>"hi"</p>} } } }` already works, but add a dedicated
  `<Show when={cond}>{...}</Show>` component and a keyed `<For each={signal_vec} key=|x| x.id>{|x| view!{...}}</For>`
  for efficient list diffing (see Task 3.1 `SignalVec`).
- **Files:** `crates/macro/src/lib.rs`, `crates/dom/src/lib.rs` (keyed list reconciler).
- **Verify:** list example with add/remove stays correct without full re-render.

---

## 2. Core reactivity upgrades (power features)

### Task 2.1: Typed read/write signals + derived `Memo`
- **Objective:** `let (count, set_count) = create_signal(0);` plus `let double = create_memo(|_| count.get()*2);`.
- **Approach:** In `crates/core`, add `create_signal<T>()` returning `(ReadSignal<T>, WriteSignal<T>)`
  (or a split API), and `create_memo`. Keep `Signal::new` for back-compat or deprecate.
- **Files:** `crates/core/src/lib.rs`.
- **Test:** unit test for memo recomputation only when deps change.
- **Verify:** `cargo test -p velo_core`.

### Task 2.2: `SignalVec` / `SignalSlice` for collections (reactive lists)
- **Objective:** Reactive arrays so `<For>` can do fine-grained inserts/removes.
- **Approach:** `Vec`-backed signal with `.push()/.remove()/.len()` that notifies subscribers with a diff.
- **Files:** `crates/core/src/lib.rs`, used by `<For>` in Task 1.4.
- **Test:** push triggers exactly one subscriber run.

### Task 2.3: Effect lifecycle / disposers & cleanup
- **Objective:** `create_effect` should return a `Disposer` and support `on_cleanup` for event-listener
  teardown (currently closures are `.forget()`-leaked — a memory concern).
- **Files:** `crates/core/src/lib.rs`, `crates/dom` event binding.
- **Verify:** component unmount removes listeners (add a `test` using a mock).

### Task 2.4: Batching / transactions
- **Objective:** `batch(|| { a.set(1); b.set(2); })` runs dependents once, not twice.
- **Files:** `crates/core/src/lib.rs` (pending-set queue).
- **Test:** two sets → one effect run.

---

## 3. New features

### Task 3.1: Server-Side Rendering (SSR) + hydration
- **Objective:** Render `view!` to an HTML string on the server; hydrate on client (no flicker, SEO).
- **Approach:** Add `crates/server` with a `render_to_string(node)` that walks `DomNode` (or a parallel
  server `DomNode` backed by a string builder instead of web-sys). Guard `web-sys` behind `#[cfg(target_arch="wasm32")]`.
  Add `velo::hydrate(root)` that attaches effects to server-rendered nodes by `data-velo-id`.
- **Files:** `crates/server/src/lib.rs`, `crates/dom` cfg-gating, `crates/velo` facade.
- **Verify:** `cargo run -p velo-ssr-demo` prints HTML; client hydrates.

### Task 3.2: Streaming SSR / Suspense
- **Objective:** `<Suspense>` boundary that streams fallback then content (async data fetching).
- **Approach:** `crates/server` stream API + `create_resource` in core.
- **Files:** `crates/core` (`create_resource`), `crates/server`, `crates/macro` (`<Suspense>`).
- **Verify:** demo with a 200ms-delayed resource shows fallback then content.

### Task 3.3: Router upgrades — nested routes, query params, `<Link>` children
- **Objective:** `view! { <Router><Route path="/" view=home/><Route path="/u/:id" view=user/></Router> }`,
  query-string parsing, `<Link to="/x">any children</Link>`.
- **Files:** `crates/router/src/lib.rs`, `crates/macro` (Route/Louter/ink components in macro), `crates/dom`.
- **Verify:** nested nav + `?q=` read works in example.

### Task 3.4: Forms & `v-model`-style two-way binding
- **Objective:** `view! { <input bind:value={ text_signal } /> }` for inputs/select/checkbox.
- **Files:** `crates/macro` (`bind:` prefix), `crates/dom` (input event → signal.set).
- **Verify:** typing updates signal; signal change updates input value.

### Task 3.5: Stores / global state
- **Objective:** `create_store!(Theme { dark: bool })` context provider + `use_store()`.
- **Files:** `crates/core` (context API: `provide_context`/`use_context`), `crates/macro` (`use_context!`).
- **Verify:** theme toggle in one component reflected in another via context.

---

## 4. Better syntax & ergonomics (polish)

### Task 4.1: `class` shorthand & spread props
- `view! { <div {..attrs} class="x" /> }` — spread a `HashMap<String,String>` of attrs.
- **Files:** `crates/macro/src/lib.rs`.

### Task 4.2: Fragment as default (no `div` wrapper for `<For>`)
- The macro currently wraps `<For>` in a `div class="contents"`. Change to a real
  `DocumentFragment` (already exists as `DomNode::fragment()`) so layout isn't polluted.
- **Files:** `crates/macro/src/lib.rs` ForLoop arm.

### Task 4.3: Better error messages from the macro
- Emit `compile_error!` with file/line context and a hint (e.g., "expected closing tag </div>").
- **Files:** `crates/macro/src/lib.rs`.

### Task 4.4: Hot Module Reloading via Trunk + dev HMR helper
- Document Trunk `build --watch`; add a `velo dev` CLI command (Task 9) wrapping trunk.

---

## 5. Developer tooling & documentation

### Task 5.1: Real `velo` CLI (repairs Task 0.1)
- **Commands:** `velo new <name>` (scaffold example), `velo dev` (trunk watch), `velo build`
  (trunk build --release), `velo serve` (axum SSR server).
- **Files:** `crates/cli/src/main.rs`, `clap`-based.
- **Verify:** `cargo run -p velo_cli -- new demo` creates a buildable project.

### Task 5.2: Docs site + API docs + examples gallery
- Expand `README.md` into a real guide (Quickstart, Reactivity model, Components, Routing, SSR).
- Add `examples/todomvc` and `examples/ssr-demo`.
- **Verify:** `cargo doc --no-deps --open` builds; README renders on GitHub.

### Task 5.3: Benchmarks vs Leptos/Yew (size + speed)
- Add `criterion` or a `wasm-size` CI step comparing `counter-spa` gzip size.
- **Files:** `benches/`, CI job.

---

## 6. Suggested implementation order (phased)

1. Phase A (stabilize): Tasks 0.1–0.3 — healthy workspace + CI. *(blocker for everything)*
2. Phase B (DX core): Tasks 1.1, 1.2, 1.3, 2.1 — auto-tracking, components, reactive attrs, typed signals.
3. Phase C (features): 2.2–2.4, 1.4, 3.3, 3.4, 3.5 — lists, lifecycle, router, forms, stores.
4. Phase D (SSR): 3.1, 3.2 — server rendering + suspense.
5. Phase E (polish/tooling): 4.x, 5.x — syntax sugar, CLI, docs, benchmarks.

## Risks / tradeoffs
- Auto-tracking in the macro risks ambiguity when an expr is a `Signal` *and* you want the struct, not its value.
  Mitigation: explicit `.get()` still supported; auto only when the whole brace is a single signal identifier/field.
- SSR doubles the DOM abstraction (server vs wasm). Mitigation: a trait `Renderer` with two impls.
- `<For>` as `DocumentFragment` changes layout assumptions — document it.
- `closure.forget()` leaks listeners; lifecycle/disposers (2.3) must land before SSR ships.

## Open questions for the user
- Prefer `Signal::new` (single handle) or Leptos-style split `(read, write)` signals? (Affects 1.1/2.1.)
- Is SSR a hard requirement now, or post-MVP? (Affects scope of Phase D.)
- Keep the `cli` crate as the real CLI, or drop it and document `trunk` directly? (Affects 0.1/5.1.)
