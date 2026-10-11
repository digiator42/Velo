# Troubleshooting & Common Gotchas

---

## 1. Calling Methods on the Wrong Signal Handle

* **Error**: `no method named 'set' found for struct 'ReadSignal<T>'`
* **Cause**: Attempting to mutate a `ReadSignal`.
* **Fix**: Use `WriteSignal` or switch to `RwSignal` / `signal()` for a combined handle.

---

## 2. Moving Captured Variables into Multiple Closures

* **Error**: `use of moved value: 'my_value'`
* **Cause**: Moving a non-`Copy` variable into more than one `move` closure.
* **Fix**: `RwSignal`, `ReadSignal`, `WriteSignal`, `Memo`, and `SignalVec`
  are all `Copy`, so reactive handles move freely — no clone needed. Only
  your own data needs a `.clone()` per closure, which the `clone!` macro
  compresses to one line:
  ```rust
  clone!(app_drop = app, app_browse = app);
  ```

---

## 3. Unused Component Warnings

* **Cause**: Component functions called only from uppercase JSX tags `<MyComponent />` may trigger `#[warn(dead_code)]`.
* **Fix**: Annotate with `#[allow(non_snake_case)]` and `#[component]`.

---

## 4. Stale Cargo Build Cache

* **Problem**: Stale artifacts in `target/` masking compiler errors.
* **Fix**: Run a clean workspace check targeting wasm:
  ```bash
  cargo check --workspace --target wasm32-unknown-unknown
  ```
