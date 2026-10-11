# Timers, SSE & Streams

Framework-level helpers for the three most common side-channel needs: delayed
/ repeated callbacks, Server-Sent Events, and incremental response streams —
all with explicit lifetime rules so nothing leaks.

---

## 1. One-Shot Timeouts (`timeout`)

`timeout(ms, f)` runs `f` once after a delay. It is **fire-and-forget**: the
browser consumes the callback after it fires, so there is nothing to hold:

```rust
use veloasm::prelude::*;

// Toast auto-dismiss
timeout(3_000, move || toast.set(None));
```

Keep the returned handle only when you need to abort:

```rust
let pending = timeout(500, move || save());
// user typed again:
pending.cancel(); // idempotent; dropping the handle does NOT cancel
```

---

## 2. Intervals (`interval`)

`interval(ms, f)` is RAII: the ticker runs while the handle is alive and is
cancelled when it drops (`#[must_use]`, so a bare statement warns):

```rust
// Runs every second while this scope is alive:
let _tick = interval(1_000, move || clock.update(|t| *t + 1));
```

For an app-lifetime ticker, `mem::forget` it:

```rust
std::mem::forget(interval(60_000, move || poll_jobs()));
```

For a component-scoped ticker, cancel it from an effect cleanup (see
[Effect Lifecycle](effect-cleanup.md#5-manual-cleanup-registration)).

`sleep(ms)` is the async cousin — a `Future` you can `.await` inside
`async` event handlers.

---

## 3. Debounced Callbacks (`debounced`)

`debounced(f, ms)` wraps a closure so rapid calls coalesce into a **single
trailing invocation** — search inputs, resize handlers, auto-scroll on
keystrokes:

```rust
let mut search = debounced(move || query.set(input.get()), 250);
// ten keystrokes in a burst => `search()` runs once, 250ms after the last
```

Dropping the wrapper cancels any still-pending invocation.

---

## 4. Server-Sent Events (`veloasm::sse::connect`)

```rust
use veloasm::prelude::*;

let stream = sse::connect("/api/events", move |raw| {
    events.update(|e| e.push(raw));
})?;
// Connection stays open while `stream` is alive; drop it to unsubscribe.
```

* Payloads are decoded to `String`; JSON payloads are stringified for you.
* `EventSource` auto-reconnects natively; `stream.is_open()` reflects state.
* Dropping `SseConnection` calls `close()`.
* `sse::on_open(&conn, || ...)` fires after each (re)connect.

---

## 5. Streaming Responses (`stream_lines`)

Read a byte `ReadableStream` (e.g. `response.body()`) line-by-line, with
UTF-8 decoded incrementally across chunk boundaries:

```rust
use veloasm::prelude::*;

let resp = fetch("/api/chat/stream").await?;
if let Some(body) = resp.body() {
    stream_lines(&body, |line| {
        log.update(|l| l.push(line));
    })
    .await?;
}
```

`LineSplitter` is the underlying reassembler, usable standalone:

```rust
let mut sp = LineSplitter::new();
assert!(sp.push("hel").is_empty());
assert_eq!(sp.push("lo\n"), vec!["hello".to_string()]);
assert_eq!(sp.finish(), None); // no trailing partial
```

---

## 6. Auto-Scroll (`scroll_to_bottom`)

```rust
scroll_to_bottom(&log_el, true); // smooth=true animates programmatic scrolls
```

Sets the container's `scroll-behavior` (only affects programmatic scrolls —
user wheeling is untouched) and pins it to the bottom.

---

## 7. Bounded Logs

Pair these helpers with `SignalVec`'s capped mutations so streams can't grow
without bound — see [Reactive Collections](../reactivity/signal-vec.md#4-bounded-collections-capped-helpers).

