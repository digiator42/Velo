# Troubleshooting & Common Gotchas

---

## 1. Calling Methods on the Wrong Signal Handle

* **Error**: `no method named 'set' found for struct 'ReadSignal<T>'`
* **Cause**: Attempting to mutate a `ReadSignal`.
* **Fix**: Use `WriteSignal` or switch to `RwSignal` / `signal()` for a combined handle.

---

## 2. Moving Captured Variables into Multiple Closures

* **Error**: `use of moved value: 'my_signal'`
* **Cause**: Moving an un-cloned variable into a closure.
* **Fix**: Clone the signal handle before the closure:
  ```rust
  let count_c1 = count.clone();
  let count_c2 = count.clone();
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
