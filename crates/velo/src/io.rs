//! Stream and scrolling helpers: incremental line decoding for
//! `ReadableStream`s (chat/SSE-style token streams) and auto-scrolling
//! containers.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Incremental splitter: feed decoded text chunks, get complete lines back.
///
/// Handles lines split across chunk boundaries and strips `\r` from
/// `\r\n`-terminated streams:
///
/// ```ignore
/// let mut splitter = LineSplitter::new();
/// assert_eq!(splitter.push("hel"), Vec::<String>::new());
/// assert_eq!(splitter.push("lo\nwor"), vec!["hello".to_string()]);
/// assert_eq!(splitter.finish(), Some("wor".to_string()));
/// ```
#[derive(Default)]
pub struct LineSplitter {
    buf: String,
}

impl LineSplitter {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    /// Feed a chunk; returns every line completed by it (without terminators).
    pub fn push(&mut self, chunk: &str) -> Vec<String> {
        self.buf.push_str(chunk);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.find('\n') {
            let mut line = self.buf[..pos].to_string();
            if line.ends_with('\r') {
                line.pop();
            }
            lines.push(line);
            self.buf.drain(..=pos);
        }
        lines
    }

    /// Flush the trailing partial line (call when the stream ends).
    pub fn finish(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            None
        } else {
            let mut rest = std::mem::take(&mut self.buf);
            if rest.ends_with('\r') {
                rest.pop();
            }
            Some(rest)
        }
    }
}

/// Options for incremental decoding: `stream: true` buffers split multibyte
/// sequences across chunk boundaries instead of emitting replacement chars.
fn stream_opts() -> web_sys::TextDecodeOptions {
    let opts = web_sys::TextDecodeOptions::new();
    opts.set_stream(true);
    opts
}

/// Read a byte `ReadableStream` (e.g. `response.body()`) to completion,
/// decoding UTF-8 incrementally (split multibyte sequences across chunks are
/// handled by `TextDecoder`) and invoking `on_line` for each complete line,
/// plus a final trailing partial line.
///
/// ```ignore
/// let resp = fetch("/api/chat/stream").await?;
/// if let Some(body) = resp.body() {
///     stream_lines(&body, |line| log.update(|l| l.push(line))).await?;
/// }
/// ```
pub async fn stream_lines(
    stream: &web_sys::ReadableStream,
    mut on_line: impl FnMut(String),
) -> Result<(), JsValue> {
    let reader: web_sys::ReadableStreamDefaultReader = stream.get_reader().dyn_into()?;
    let decoder = web_sys::TextDecoder::new()?;
    let mut splitter = LineSplitter::new();
    let opts = stream_opts();

    loop {
        let value = JsFuture::from(reader.read()).await?;
        let done = js_sys::Reflect::get(&value, &JsValue::from_str("done"))?
            .as_bool()
            .unwrap_or(true);
        if done {
            // Flush any decoder-internal bytes, then the splitter tail.
            if let Ok(tail) = decoder.decode() {
                for line in splitter.push(&tail) {
                    on_line(line);
                }
            }
            if let Some(last) = splitter.finish() {
                on_line(last);
            }
            break;
        }
        let chunk = js_sys::Reflect::get(&value, &JsValue::from_str("value"))?;
        if chunk.is_undefined() || chunk.is_null() {
            continue;
        }
        let text = decoder
            .decode_with_js_u8_array_and_options(&chunk.dyn_into::<js_sys::Uint8Array>()?, &opts)?;
        for line in splitter.push(&text) {
            on_line(line);
        }
    }
    Ok(())
}

/// Scroll an element (typically a scrollable container like a chat log) to
/// its bottom. `smooth` enables animated scrolling by setting the element's
/// `scroll-behavior` (only affects programmatic scrolls, not user wheeling).
///
/// ```ignore
/// scroll_to_bottom(&log_el, true);
/// ```
pub fn scroll_to_bottom(el: &web_sys::Element, smooth: bool) {
    if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
        if smooth {
            let _ = html.style().set_property("scroll-behavior", "smooth");
        }
        html.set_scroll_top(html.scroll_height());
    }
}
