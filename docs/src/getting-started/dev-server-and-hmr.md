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
location that tells you what to fix. To keep those diagnostics on screen, drop
Velo's dev overlay onto any `index.html`:

```html
<script src="velo-error-overlay.js"></script>
```

The template ships at `docs/templates/velo-error-overlay.js` and is copied into
the examples (`examples/error-overlay/`, `examples/blog/`). It:

- Subscribes to Trunk's WebSocket alongside Trunk's own script.
- On `buildFailure`, renders a styled panel with the full Rust error and the
  parsed `file:line:col` badge, and hides Trunk's minimal overlay.
- On a successful `reload`, the page (and panel) refresh automatically.

The overlay is **dev-only by construction**: it only opens a connection when
the page is actually served by `trunk serve`. Behind a plain `trunk build` +
static server the WebSocket never connects and the page is untouched, so the
tag is safe to keep around.

### Test-driving the overlay

The dedicated example is `examples/error-overlay`:

```bash
trunk serve --watch --open
```

Then break `src/lib.rs` (e.g. delete a closing brace) — the panel appears with
the error and location. Fix it and save — the page reloads cleanly.

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
