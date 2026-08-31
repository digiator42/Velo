# Performance & WASM Binary Size

Velo is engineered for instant load times and tiny binary footprints.

---

## 1. Release Profile Tuning

Ensure your root `Cargo.toml` contains size-tuned profile settings:

```toml
[profile.release]
opt-level = "z"     # Optimize aggressively for binary size
lto = true          # Full Link-Time Optimization across all crates
codegen-units = 1   # Single codegen unit to maximize dead code elimination
panic = "abort"     # Strip panic landing pads and backtraces
strip = true        # Strip debug symbols and symbol tables
```

---

## 2. Using Trunk's Release Build

Compile your production assets with:

```bash
trunk build --release
```

Trunk will run `wasm-opt` automatically if installed, shrinking the WebAssembly bundle even further.

---

## 3. High-Frequency State Updates

Because Velo avoids Virtual DOM diffing, it can handle thousands of reactive updates per second without blocking the main browser thread.

To perform rapid mutations efficiently:
* Use `batch(|| { ... })` to synchronize multiple signal updates into a single notification.
* Use `SignalVec::with_mut(|| { ... })` for bulk collection mutations.
