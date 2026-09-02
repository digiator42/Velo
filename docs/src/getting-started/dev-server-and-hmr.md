# Dev Server, Error Overlay & HMR

Velo leans on [Trunk](https://trunkrs.dev) for the dev loop. Trunk compiles your
Rust to WASM, serves `index.html`, and watches your files — when you save a
change it rebuilds and reloads the browser for you.

```bash
# one-shot build + serve (no watching)
trunk serve

# watch for changes and auto-reload the browser (the dev default)
trunk serve --watch
```

Trunk serves out of `dist/` at `http://127.0.0.1:8080` (change with `--port`),
and injects a small script that talks to a WebSocket at
`/.well-known/trunk/ws`, pushing build results to the page.

## The Velo dev error overlay

On a failed build Trunk shows a minimal grey "Build failure" panel, but you
lose the actual Rust diagnostic — the message with the `--> file:line:col`
location that tells you what to fix. Velo ships a **built-in** dev overlay so
you never have to wire anything up:

- It is installed automatically by `mount()` on every Velo app.
- It subscribes to Trunk's WebSocket alongside Trunk's own script.
- On `buildFailure`, it renders a styled panel with the build reason and hides
  Trunk's minimal overlay.
- On a successful `reload`, the page (and panel) refresh automatically.

**Zero per-project setup.** There is no `<script>` tag, no asset copy, no
config to add — a fresh project's `index.html` stays untouched because the
overlay ships compiled into the `velo` crate. It is dev-only by construction:
it only connects when the page is actually served by `trunk serve`; behind a
plain `trunk build` + static server the WebSocket never connects and the page
is untouched.

### How the built-in works

The overlay logic lives as a JS constant inside `crates/velo/src/lib.rs`
(`DEV_OVERLAY_JS`) and is evaluated once on mount via `install_dev_overlay()`.
A cross-project editable stand-alone copy of the same overlay is kept at
`docs/templates/velo-error-overlay.js` for reference — the compiled built-in is
the canonical, always-in-sync version you get for free.

### Test-driving the overlay

The dedicated example is `examples/error-overlay` — it needs no overlay script
of its own, exactly like a real new project:

```bash
trunk serve --watch --open
```

Then break `src/lib.rs` (e.g. delete a closing brace) — the panel appears with
the error. Fix it and save — the page reloads cleanly.

## HMR & state-survival caveats

Trunk's dev reload is a **full page reload** (it re-instantates the WASM), not
fine-grained hot module replacement. That means:

- All in-memory application state resets on every save.
- It is the correct, predictable behavior for an SPA: a fresh mount is
  guaranteed to reflect the new code.

If you need state to survive a dev reload, persist it (`localStorage`, the
server, or a hardcoded seed), or use the Reactivity-driven live views in the
[High-Frequency Live Dashboard](../examples/realtime-dashboard.md) for a
closer-to-HMR feel within a single session.
