# Introduction to Velo

**Velo** is a modern, high-performance, client-side WebAssembly framework for building Single Page Applications (SPAs) in Rust.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           NO VIRTUAL DOM                                │
│                                                                         │
│   State Change (Signal Mutation) ───► Targeted DOM Text/Attr Update     │
│                                                                         │
│   No diffing algorithm • No VDOM trees • Native Surgical Precision      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Why Velo?

In traditional web frameworks (like React or standard Virtual DOM-based WASM frameworks), state mutations trigger a re-render of an entire component tree. The framework generates a Virtual DOM tree in memory, diffs it against the previous tree, and calculates patches for the real browser DOM.

**Velo completely removes the Virtual DOM layer.**

Instead, Velo uses **fine-grained reactivity**:
1. Your application runs once to construct the initial DOM layout.
2. Reactive signals establish direct listener relationships with specific DOM text nodes, element attributes, and class lists.
3. When a signal changes, **only the single native DOM node that depends on that signal is updated** directly via browser APIs.

---

## Core Pillars

* **⚡ Blazing Fast**: No Virtual DOM diffing overhead. State updates execute in sub-millisecond timeframes.
* **📦 Minimal Binary Size**: Built with release optimizations (`opt-level = "z"`, cross-crate LTO) to produce compact WebAssembly bundles.
* **🪄 Clean Syntax Sugar**: Write familiar HTML/JSX markup via the `view!` declarative macro, with automatic signal unwrapping and two-way form bindings (`bind:value`).
* **🧩 Component Architecture**: Write clean, ergonomic components using the `#[component]` attribute macro.
* **🧭 Built-In Router**: SPA routing with parameterized path matching, query string extraction, and client-side history navigation.
* **🌐 Shared Context**: Global state management and dependency injection without prop drilling.

---

## Next Steps

Jump into the [Installation & Toolchain](getting-started/installation.md) guide to set up your environment, or head straight to the [Quickstart Guide](getting-started/quickstart.md) to build your first Velo application in under 5 minutes!
