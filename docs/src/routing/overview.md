# Client-Side Routing Overview

Velo includes a built-in client-side router (part of the unified `velo` crate: `Router`, `Route`, `Link`, `FRouter`) designed specifically for Single Page Applications running in WebAssembly.

---

## 1. How the Router Works

* **HTML5 History API**: Uses `window.history.pushState` to transition URLs without full browser page reloads.
* **Reactive Route Signals**: The current path and query strings are backed by reactive signals (`CURRENT_PATH`, `CURRENT_QUERY`). When the URL changes (via `<Link>` clicks or browser back/forward buttons), the router effect re-evaluates and switches views surgically.
* **Pattern Matching**: Supports exact paths (`/dashboard`), dynamic path parameters (`/users/:id`), and catch-all wildcard routes (`/**`).
