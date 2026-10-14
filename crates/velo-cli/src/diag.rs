//! Dev-time Rust diagnostics sidecar (`veloasm dev` only).
//!
//! Trunk's `buildFailure` WebSocket message carries only a pipeline summary
//! (`cargo returned exit 101`) — never the rustc spans. To deliver the
//! promised `file.rs:line:col` panel, `veloasm dev` pipes Trunk's stderr,
//! parses each `error` block (message + `-->` spans), and serves the latest
//! failure as JSON. The built-in dev overlay fetches it on `buildFailure`
//! and falls back to the summary when no sidecar answers.
//!
//! Endpoint: `GET http://127.0.0.1:3197/_veloasm/diagnostics`
//! ```json
//! {"errors": [{"message": "mismatched types",
//!               "spans": [{"file": "src/x.rs", "line": 27, "col": 24}],
//!               "detail": "full rustc block (capped)"}]}
//! ```

use std::borrow::Cow;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::ChildStderr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Fixed loopback port for the diagnostics sidecar.
pub const DIAG_PORT: u16 = 3197;
/// Path served by the sidecar.
pub const DIAG_PATH: &str = "/_veloasm/diagnostics";
/// Max errors kept per failed build; max chars of rustc block per error.
const MAX_ERRORS: usize = 20;
const MAX_DETAIL: usize = 4000;

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
    pub spans: Vec<Span>,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct DiagState {
    errors: Mutex<Vec<CompileError>>,
}

impl DiagState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn replace(&self, errors: Vec<CompileError>) {
        *self.errors.lock().unwrap() = errors;
    }

    pub fn snapshot(&self) -> Vec<CompileError> {
        self.errors.lock().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences (`\x1b[...m`) so spans parse cleanly whether
/// or not cargo colorizes its output.
fn strip_ansi(s: &str) -> Cow<'_, str> {
    if !s.contains('\x1b') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip CSI sequences: ESC [ ... final-byte(A-Za-z).
            let mut it = chars.clone();
            match it.next() {
                Some('[') => {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Split off a trailing `:digits` segment: `("src/x.rs:27", "24")`.
fn rsplit_num(s: &str) -> Option<(&str, u32)> {
    let idx = s.rfind(':')?;
    let num: u32 = s[idx + 1..].trim().parse().ok()?;
    Some((s[..idx].trim_end(), num))
}

/// Parse a rustc location from a `-->` line: `--> src/x.rs:27:24`.
/// Works with Windows drive prefixes (`C:\x.rs:1:2`) because only trailing
/// numeric segments are consumed.
fn parse_span(line: &str) -> Option<Span> {
    let rest = line.split("-->").nth(1)?.trim();
    let (rest, col) = rsplit_num(rest)?;
    let (file, line) = rsplit_num(rest)?;
    if file.is_empty() {
        return None;
    }
    Some(Span {
        file: file.to_string(),
        line,
        col,
    })
}

fn is_error_start(line: &str) -> bool {
    line.starts_with("error") && {
        let rest = &line["error".len()..];
        rest.is_empty() || rest.starts_with(':') || rest.starts_with('[')
    }
}

/// Summary lines cargo prints around real diagnostics; kept out so the panel
/// shows actionable errors only.
fn is_noise(line: &str) -> bool {
    line.starts_with("error: could not compile")
        || line.starts_with("error: aborting")
        || line.starts_with("warning:")
}

/// Incremental parser cursor: keep one across `feed_lines` calls while a
/// build streams, then drop it. (Public so the streaming entry point can name
/// it; construct with `Builder::default()`.)
#[derive(Debug, Default)]
pub struct Builder {
    message: String,
    detail: String,
    spans: Vec<Span>,
}

impl Builder {
    fn finish(self) -> Option<CompileError> {
        if self.message.is_empty() && self.spans.is_empty() && self.detail.is_empty() {
            return None;
        }
        let mut detail = self.detail;
        if detail.len() > MAX_DETAIL {
            detail.truncate(MAX_DETAIL);
            detail.push_str("\n…(truncated)");
        }
        Some(CompileError {
            message: self.message,
            spans: self.spans,
            detail,
        })
    }
}

/// Fold stderr lines into completed `CompileError`s. A fresh `Compiling`
/// marker means a new build started, so previously collected errors are
/// stale and dropped. Call with `flush = true` at EOF.
pub fn feed_lines<'a, I>(lines: I, flush: bool, out: &mut Vec<CompileError>, cur: &mut Builder)
where
    I: IntoIterator<Item = &'a str>,
{
    for raw in lines {
        let line = strip_ansi(raw);
        let trimmed = line.trim_end().to_string();
        if trimmed.trim_start().starts_with("Compiling ") {
            out.clear();
            *cur = Builder::default();
            continue;
        }
        if is_noise(&trimmed) {
            continue;
        }
        if is_error_start(&trimmed) {
            if let Some(done) = std::mem::take(cur).finish() {
                out.push(done);
                if out.len() >= MAX_ERRORS {
                    *cur = Builder::default();
                    continue;
                }
            }
            // `error`, `error: msg`, `error[E0308]: msg`.
            let after = trimmed["error".len()..].trim_start();
            let after = after.strip_prefix(':').unwrap_or(after).trim();
            let message = if let Some(end) = after.find("]:") {
                after[end + 2..].trim().to_string()
            } else {
                after.to_string()
            };
            cur.message = if message.is_empty() {
                "compile error".to_string()
            } else {
                message
            };
            cur.detail.push_str(&trimmed);
            cur.detail.push('\n');
        } else if !cur.message.is_empty() || !cur.detail.is_empty() {
            if trimmed.is_empty() {
                // Blank line ends the current block.
                if let Some(done) = std::mem::take(cur).finish() {
                    out.push(done);
                    if out.len() >= MAX_ERRORS {
                        *cur = Builder::default();
                    }
                }
            } else {
                if let Some(span) = parse_span(&trimmed) {
                    cur.spans.push(span);
                }
                cur.detail.push_str(&trimmed);
                cur.detail.push('\n');
            }
        }
    }
    if flush {
        if let Some(done) = std::mem::take(cur).finish() {
            out.push(done);
        }
    }
}

// ---------------------------------------------------------------------------
// JSON (hand-rolled: the CLI keeps zero serialization dependencies)
// ---------------------------------------------------------------------------

fn esc_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

pub fn to_json(errors: &[CompileError]) -> String {
    let mut out = String::from("{\"errors\":[");
    for (i, e) in errors.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"message\":\"");
        esc_into(&mut out, &e.message);
        out.push_str("\",\"spans\":[");
        for (j, s) in e.spans.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str("{\"file\":\"");
            esc_into(&mut out, &s.file);
            out.push_str("\",\"line\":");
            out.push_str(&s.line.to_string());
            out.push_str(",\"col\":");
            out.push_str(&s.col.to_string());
            out.push('}');
        }
        out.push_str("],\"detail\":\"");
        esc_into(&mut out, &e.detail);
        out.push_str("\"}");
    }
    out.push_str("]}");
    out
}

// ---------------------------------------------------------------------------
// Pump + sidecar server
// ---------------------------------------------------------------------------

/// Paint one echoed line for the terminal: rustc `error` headers go bright
/// red, `warning` headers yellow. Everything else passes through byte-for-
/// byte (including any colors cargo emitted itself). Honors `NO_COLOR`.
fn paint_line<'a>(line: &'a str, color: bool) -> Cow<'a, str> {
    if !color {
        return Cow::Borrowed(line);
    }
    let stripped = strip_ansi(line);
    // Already colorized lines keep their own colors; only uncolored error /
    // warning headers get painted so failures pop in monochrome pipes.
    if stripped == line {
        let trimmed = stripped.trim_start();
        if is_error_start(trimmed) {
            return Cow::Owned(format!("\x1b[1;31m{line}\x1b[0m"));
        }
        if trimmed.starts_with("warning:") {
            return Cow::Owned(format!("\x1b[1;33m{line}\x1b[0m"));
        }
    }
    Cow::Borrowed(line)
}

/// Read Trunk's stderr line by line on a worker thread: echo every line to
/// our own stderr (so the terminal stays live) and fold `error` blocks into
/// `state` for the overlay to fetch.
pub fn pump_trunk_stderr(stderr: ChildStderr, state: Arc<DiagState>) {
    // Piped output looks monochrome unless asked otherwise; the caller sets
    // `CARGO_TERM_COLOR`/`CLICOLOR_FORCE` on the Trunk child for rich colors.
    // Either way, error headers are painted here so they never stay white.
    let color = std::env::var_os("NO_COLOR").is_none();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut out: Vec<CompileError> = Vec::new();
        let mut cur = Builder::default();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            eprintln!("{}", paint_line(&line, color));
            feed_lines([line.as_str()], false, &mut out, &mut cur);
            state.replace(out.clone());
        }
        feed_lines(std::iter::empty::<&str>(), true, &mut out, &mut cur);
        state.replace(out);
    });
}

/// Serve `GET /_veloasm/diagnostics` on the loopback sidecar port.
/// Best-effort: returns the bind error so `dev` can warn and continue
/// without diagnostics (the overlay then falls back to Trunk's summary).
pub fn serve(state: Arc<DiagState>) -> std::io::Result<()> {
    serve_on(DIAG_PORT, state).map(|_| ())
}

/// Like [`serve`], but on an explicit port, returning the bound port.
/// Port `0` asks the OS for a free one — used by tests so they never clash
/// with a running dev server.
pub fn serve_on(port: u16, state: Arc<DiagState>) -> std::io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let bound = listener.local_addr()?.port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut head = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        head.extend_from_slice(&chunk[..n]);
                        if head.windows(4).any(|w| w == b"\r\n\r\n") || head.len() > 8192 {
                            break;
                        }
                    }
                }
            }
            let head = String::from_utf8_lossy(&head);
            let mut parts = head.split_whitespace();
            let method = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("/");
            let (status, body) = if method == "GET"
                && (path == DIAG_PATH || path.starts_with(&format!("{DIAG_PATH}?")))
            {
                ("200 OK", to_json(&state.snapshot()))
            } else {
                ("404 Not Found", "{\"error\":\"not_found\"}".to_string())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    Ok(bound)
}

// ---------------------------------------------------------------------------
// Tests (host-runnable: pure parser + JSON, no browser needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const E0308: &str = "error[E0308]: mismatched types\n   --> src\\components\\drop_zone.rs:55:5\n    |\n 55 | /     view! {\n    | |_____^ expected `String`, found `&str`\n";

    fn collect(input: &str) -> Vec<CompileError> {
        let mut out = Vec::new();
        let mut cur = Builder::default();
        let lines: Vec<&str> = input.lines().collect();
        feed_lines(lines, true, &mut out, &mut cur);
        out
    }

    #[test]
    fn parses_message_and_windows_span() {
        let errors = collect(E0308);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "mismatched types");
        assert_eq!(
            errors[0].spans,
            vec![Span {
                file: "src\\components\\drop_zone.rs".to_string(),
                line: 55,
                col: 5
            }]
        );
    }

    #[test]
    fn multiple_errors_each_keep_spans() {
        let input = "error: expected ident\n  --> src\\a.rs:27:24\n\nerror[E0382]: use of moved value\n  --> src\\b.rs:56:5\n";
        let errors = collect(input);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].spans[0].line, 27);
        assert_eq!(errors[1].message, "use of moved value");
        assert_eq!(errors[1].spans[0].file, "src\\b.rs");
    }

    #[test]
    fn compiling_marker_drops_stale_errors() {
        let mut out = Vec::new();
        let mut cur = Builder::default();
        feed_lines(
            ["error: expected ident", "  --> src\\a.rs:1:2", ""],
            false,
            &mut out,
            &mut cur,
        );
        assert_eq!(out.len(), 1);
        feed_lines(
            ["   Compiling gritscan-frontend v0.1.0"],
            false,
            &mut out,
            &mut cur,
        );
        assert!(out.is_empty(), "stale errors must clear on rebuild");
    }

    #[test]
    fn paint_line_highlights_error_and_warning_headers() {
        assert_eq!(
            paint_line("error[E0308]: mismatched types", true),
            "\x1b[1;31merror[E0308]: mismatched types\x1b[0m"
        );
        assert_eq!(
            paint_line("warning: unused variable", true),
            "\x1b[1;33mwarning: unused variable\x1b[0m"
        );
        // Plain lines and already-colored lines pass through untouched.
        assert_eq!(paint_line("   Compiling x", true), "   Compiling x");
        assert_eq!(
            paint_line("\x1b[31merror: boom\x1b[0m", true),
            "\x1b[31merror: boom\x1b[0m"
        );
        // NO_COLOR equivalent: off switch disables all painting.
        assert_eq!(paint_line("error: boom", false), "error: boom");
    }

    #[test]
    fn noise_lines_are_skipped() {
        let errors = collect(
            "error: could not compile `x` due to 2 previous errors\nerror: aborting due to previous error\n",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn ansi_codes_do_not_break_spans() {
        let input = "\x1b[31;1merror\x1b[0m\x1b[1m: expected ident\x1b[0m\n   \x1b[1m-->\x1b[0m src\\a.rs:26:64\n";
        let errors = collect(input);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "expected ident");
        assert_eq!(
            errors[0].spans,
            vec![Span {
                file: "src\\a.rs".to_string(),
                line: 26,
                col: 64
            }]
        );
    }

    #[test]
    fn sidecar_serves_parsed_errors_over_http() {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let state = DiagState::new();
        state.replace(collect(
            "error[E0308]: mismatched types\n   --> src\\x.rs:27:24\n",
        ));
        let port = serve_on(0, state).expect("sidecar bind");
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("sidecar connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET /_veloasm/diagnostics HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut body = String::new();
        stream.read_to_string(&mut body).unwrap();
        assert!(body.starts_with("HTTP/1.1 200 OK"), "status line: {body}");
        assert!(
            body.contains("Access-Control-Allow-Origin: *"),
            "CORS header for the cross-port overlay fetch"
        );
        let payload = body.split("\r\n\r\n").nth(1).unwrap_or("");
        assert!(payload.contains("\"message\":\"mismatched types\""), "{payload}");
        assert!(
            payload.contains("\"file\":\"src\\\\x.rs\",\"line\":27,\"col\":24"),
            "{payload}"
        );
    }

    #[test]
    fn json_escapes_quotes_and_newlines() {
        let errors = vec![CompileError {
            message: "expected `\"` found".to_string(),
            spans: vec![Span {
                file: "a.rs".to_string(),
                line: 1,
                col: 2,
            }],
            detail: "line1\nline2".to_string(),
        }];
        assert_eq!(
            to_json(&errors),
            "{\"errors\":[{\"message\":\"expected `\\\"` found\",\"spans\":[{\"file\":\"a.rs\",\"line\":1,\"col\":2}],\"detail\":\"line1\\nline2\"}]}"
        );
    }
}
