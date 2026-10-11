//! Server-Sent Events client (`veloasm::sse::connect`).
//!
//! Thin wrapper over [`web_sys::EventSource`]: auto-reconnects natively,
//! routes `data` payloads (string or JSON) into a Rust callback, and closes
//! the connection when the returned handle drops.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// An open SSE connection. Dropping it closes the underlying `EventSource`.
pub struct SseConnection {
    es: web_sys::EventSource,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

impl SseConnection {
    /// Close the connection now (also done automatically on drop).
    pub fn close(&self) {
        self.es.close();
    }

    /// Whether the underlying `EventSource` is currently open.
    pub fn is_open(&self) -> bool {
        self.es.ready_state() == web_sys::EventSource::OPEN
    }
}

impl Drop for SseConnection {
    fn drop(&mut self) {
        self.es.close();
    }
}

/// Connect to an SSE endpoint. Every message's `data` field is decoded to a
/// `String` (JSON payloads are stringified) and passed to `on_message`.
///
/// ```ignore
/// let stream = sse::connect("/api/chat/events", move |raw| {
///     log.update(|l| l.push(raw));
/// })?;
/// // Connection stays open while `stream` is alive; drop it to unsubscribe.
/// ```
pub fn connect(
    url: &str,
    mut on_message: impl FnMut(String) + 'static,
) -> Result<SseConnection, JsValue> {
    let es = web_sys::EventSource::new(url)?;

    let cb = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        let payload = e.data().as_string().or_else(|| {
            js_sys::JSON::stringify(&e.data())
                .ok()
                .and_then(|v| v.as_string())
        });
        if let Some(text) = payload {
            on_message(text);
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);

    es.set_onmessage(Some(cb.as_ref().unchecked_ref()));
    Ok(SseConnection {
        es,
        _on_message: cb,
    })
}

/// Fire `f` whenever the connection opens (after each successful connect /
/// reconnect). Call alongside [`connect`].
pub fn on_open(conn: &SseConnection, mut f: impl FnMut() + 'static) -> OpenHandle {
    let cb = Closure::wrap(Box::new(move || f()) as Box<dyn FnMut()>);
    conn.es.set_onopen(Some(cb.as_ref().unchecked_ref()));
    OpenHandle {
        _cb: Rc::new(RefCell::new(Some(cb))),
    }
}

/// Keeps an [`on_open`] callback alive; dropping it clears the handler.
pub struct OpenHandle {
    _cb: Rc<RefCell<Option<Closure<dyn FnMut()>>>>,
}

