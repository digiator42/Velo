/*!
 * Velo dev error overlay — the canonical template.
 *
 * Drop this single <script> into your index.html and it renders Trunk's
 * compile failures as a styled on-page panel during `trunk serve --watch`:
 *
 *     <script src="velo-error-overlay.js"></script>
 *
 * How it works
 * ------------
 * `trunk serve` (dev only) pushes build results to the browser over a
 * WebSocket at `/.well-known/trunk/ws`.
 *   - `{ type: "buildFailure", data: { reason } }` — the Rust compile failed;
 *     we show `reason` in a styled panel instead of a dead tab.
 *   - `{ type: "reload" }` — the build recovered; the page reloads and the
 *     panel disappears with it.
 *
 * NOTE on `reason`: Trunk's WebSocket only carries a pipeline summary (e.g.
 * "error from build pipeline ... cargo returned a bad status: exit code 101").
 * The full Rust diagnostic with its `--> file.rs:line:col` span is printed to
 * the `trunk serve` terminal, not streamed to the page — so the panel points
 * you there, and only shows `file:line:col` when it happens to appear in the
 * message. (Velo's own error boundaries catch runtime faults in-app; this
 * overlay is strictly for dev-time compile failures.)
 *
 * This script is deliberately inert when the app is NOT served by Trunk in
 * dev mode (e.g. a `trunk build` + any static server): the WebSocket never
 * opens and the page is untouched — so it is safe to ship the tag everywhere.
 *
 * Trunk also injects its own minimal "Build failure" overlay; we hide it via a
 * style fingerprint so only this panel shows.
 *
 * copy into any example that `trunk serve`s: keep the exact sync + a matching
 * `<link data-trunk rel="copy-file" href="velo-error-overlay.js"/>` directive
 * in index.html so it lands in dist/.
 */
(() => {
  if (window.__veloDevOverlay) return;
  window.__veloDevOverlay = true;

  const guardStyle = document.createElement("style");
  guardStyle.textContent =
    'div[style*="rgba(222, 222, 222, 0.5)"]{display:none !important;}';
  document.head.appendChild(guardStyle);

  let panel = null;

  const WSLocation = () => {
    const proto = location.protocol === "https:" ? "wss://" : "ws://";
    return `${proto}${location.host}/.well-known/trunk/ws`;
  };

  const buildPanel = () => {
    const root = document.createElement("div");
    root.id = "velo-dev-overlay";
    root.setAttribute(
      "style",
      [
        "position:fixed",
        "inset:0",
        "z-index:2147483000",
        "display:flex",
        "align-items:center",
        "justify-content:center",
        "padding:2rem",
        "background:rgba(2,6,23,.72)",
        "backdrop-filter:blur(6px)",
        "font-family:system-ui,-apple-system,sans-serif",
        "color:#e2e8f0",
      ].join(";"),
    );

    const card = document.createElement("div");
    card.setAttribute(
      "style",
      [
        "max-width:min(880px,100%)",
        "width:100%",
        "max-height:85vh",
        "overflow:auto",
        "border:1px solid #7f1d1d",
        "border-radius:14px",
        "background:#111827",
        "box-shadow:0 24px 60px rgba(0,0,0,.5)",
      ].join(";"),
    );

    const head = document.createElement("div");
    head.setAttribute(
      "style",
      [
        "display:flex",
        "align-items:center",
        "gap:.75rem",
        "padding:1rem 1.25rem",
        "border-bottom:1px solid #374151",
        "background:#1f2937",
        "border-radius:14px 14px 0 0",
      ].join(";"),
    );

    const icon = document.createElement("span");
    icon.innerHTML =
      '<svg width="22" height="22" viewBox="0 0 16 16" fill="none">' +
      '<path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566z"' +
      ' fill="#f87171"/><path d="M8 5.5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 6.495A.905.905 0 0 1 8 5.5zm.002 5.5a1 1 0 1 1 0 2 1 1 0 0 1 0-2z" fill="#111827"/></svg>';

    const title = document.createElement("span");
    title.textContent = "Build failed";
    title.setAttribute("style", "font-size:1.05rem;font-weight:700;color:#fca5a5");

    const close = document.createElement("button");
    close.type = "button";
    close.setAttribute("aria-label", "Dismiss");
    close.textContent = "\u00d7";
    close.setAttribute(
      "style",
      [
        "margin-left:auto",
        "background:transparent",
        "border:none",
        "color:#94a3b8",
        "font-size:1.5rem",
        "cursor:pointer",
        "line-height:1",
        "padding:.25rem .6rem",
        "border-radius:8px",
      ].join(";"),
    );
    close.addEventListener("mouseenter", () => (close.style.color = "#f1f5f9"));
    close.addEventListener("mouseleave", () => (close.style.color = "#94a3b8"));

    const loc = document.createElement("div");
    loc.id = "velo-dev-overlay-loc";
    loc.setAttribute(
      "style",
      [
        "display:inline-block",
        "margin:.9rem 1.25rem 0",
        "padding:.2rem .7rem",
        "border-radius:999px",
        "background:#312e81",
        "border:1px solid #4338ca",
        "color:#c7d2fe",
        "font:600 .8rem ui-monospace,SFMono-Regular,Menlo,monospace",
      ].join(";"),
    );

    const msg = document.createElement("pre");
    msg.id = "velo-dev-overlay-msg";
    msg.setAttribute(
      "style",
      [
        "margin:1rem 1.25rem 1.25rem",
        "white-space:pre-wrap",
        "word-break:break-word",
        "font:0.83rem/1.55 ui-monospace,SFMono-Regular,Menlo,monospace",
        "color:#fbbf24",
      ].join(";"),
    );

    const foot = document.createElement("div");
    foot.setAttribute(
      "style",
      [
        "padding:.6rem 1.25rem",
        "border-top:1px solid #1f2937",
        "color:#64748b",
        "font-size:.78rem",
      ].join(";"),
    );
    foot.textContent =
      "Velo dev overlay \u00b7 trunk serve --watch \u00b7 the full diagnostic with `--> file.rs:line:col` is in the trunk server terminal; save the fix and the page reloads automatically.";

    head.append(icon, title, close);
    card.append(head, loc, msg, foot);
    root.appendChild(card);
    document.body.appendChild(root);

    const dismiss = () => {
      root.remove();
      panel = null;
    };
    close.addEventListener("click", dismiss);
    root.addEventListener("keydown", dismiss);
    root.addEventListener("click", (ev) => {
      if (ev.target === root) dismiss();
    });
    return root;
  };

  const locationOf = (reason) => {
    const m = /-->[ \t]+([^\s]+):(\d+):(\d+)/.exec(reason || "");
    return m ? `${m[1]}:${m[2]}:${m[3]}` : null;
  };

  let reloading = false;
  const tryReload = () => {
    if (reloading) return;
    reloading = true;
    window.location.reload();
    setTimeout(() => (reloading = false), 1000);
  };

  const showFailure = ({ reason }) => {
    console.error("Velo dev overlay: build failed\n" + reason);
    if (!panel) panel = buildPanel();
    const loc = document.getElementById("velo-dev-overlay-loc");
    const msg = document.getElementById("velo-dev-overlay-msg");
    if (loc) {
      const at = locationOf(reason);
      loc.style.display = at ? "inline-block" : "none";
      loc.textContent = at || "";
    }
    if (msg) msg.textContent = reason || "Unknown build failure";
  };

  const connect = () => {
    let ws;
    try {
      ws = new WebSocket(WSLocation());
    } catch (_e) {
      return;
    }
    const retry = () => setTimeout(connect, 5000);
    ws.onopen = () => {
      ws.onclose = () => {
        // We were connected to a real dev server; if it comes back reload.
        tryReload();
        retry();
      };
    };
    ws.onerror = () => ws.close();
    ws.onclose = () => {
      if (!ws.retried) {
        ws.retried = true;
        retry();
      }
    };
    ws.onmessage = (ev) => {
      let msg;
      try {
        msg = JSON.parse(ev.data);
      } catch (_e) {
        return;
      }
      if (msg.type === "reload") tryReload();
      else if (msg.type === "buildFailure") showFailure(msg.data);
    };
  };

  try {
    connect();
  } catch (_e) {
    /* never let the overlay take down the app */
  }
})();