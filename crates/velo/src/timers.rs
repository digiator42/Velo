//! Timer utilities: one-shot timeouts, intervals, and debounced callbacks.
//!
//! All three wrap `window.setTimeout` / `window.setInterval` with explicit,
//! leak-free lifetime rules:
//!
//! - [`timeout`] is fire-and-forget: nothing to hold, nothing leaks. Call
//!   [`TimeoutHandle::cancel`] (keep the handle) if you need to abort it.
//! - [`interval`] is RAII: the ticker runs while its handle is alive and is
//!   cancelled when the handle drops (`#[must_use]`, so a bare statement warns).
//!   For an app-lifetime ticker use `std::mem::forget(interval(..))`.
//! - [`debounced`] returns a `FnMut` wrapper that coalesces rapid calls into a
//!   single trailing invocation.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

fn window() -> web_sys::Window {
    web_sys::window()
        .expect("veloasm: No global window found. Are you running in a browser environment?")
}

fn clamp_ms(ms: u32) -> i32 {
    ms.min(i32::MAX as u32) as i32
}

/// Handle for a pending [`timeout`]. Dropping it does **not** cancel the
/// timeout (fire-and-forget is the default); call [`TimeoutHandle::cancel`]
/// to abort it before it fires.
pub struct TimeoutHandle {
    id: Rc<Cell<Option<i32>>>,
}

impl TimeoutHandle {
    /// Cancel the pending timeout if it hasn't fired yet. Idempotent.
    pub fn cancel(&self) {
        if let Some(id) = self.id.take() {
            window().clear_timeout_with_handle(id);
        }
    }
}

/// Run `f` once after `ms` milliseconds.
///
/// The callback is consumed by the browser after it fires, so there is
/// nothing to hold or clean up:
///
/// ```ignore
/// timeout(300, move || toast.set(Some("Saved".into())));
/// ```
pub fn timeout(ms: u32, f: impl FnOnce() + 'static) -> TimeoutHandle {
    let id: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
    let id_for_cb = Rc::clone(&id);
    // `once_into_js` hands ownership of `f` to the browser: the closure runs
    // at most once and frees its captured state when invoked.
    let cb = Closure::once_into_js(move || {
        id_for_cb.set(None); // fired: cancel() later becomes a no-op
        f();
    });
    let handle = window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.unchecked_ref::<js_sys::Function>(),
            clamp_ms(ms),
        )
        .expect("veloasm: window.setTimeout failed");
    id.set(Some(handle));
    TimeoutHandle { id }
}

/// Handle for an [`interval`] ticker. Cancels the interval when dropped
/// (RAII) — bind it (`let _tick = interval(..)`) to keep the ticker alive,
/// or call [`IntervalHandle::cancel`] to stop it early. Idempotent.
#[must_use = "dropping the handle immediately cancels the interval"]
pub struct IntervalHandle {
    id: Option<i32>,
    _cb: Closure<dyn FnMut()>,
}

impl IntervalHandle {
    /// Stop the interval. Idempotent; dropping the handle afterwards is safe.
    pub fn cancel(&mut self) {
        if let Some(id) = self.id.take() {
            window().clear_interval_with_handle(id);
        }
    }
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Run `f` every `ms` milliseconds until the returned handle is dropped or
/// cancelled.
///
/// ```ignore
/// // Component-scoped ticker: cancel it when the effect's cleanup runs.
/// let mut tick = interval(1_000, move || seconds.update(|s| *s += 1));
/// create_effect_with_cleanup(move || { let _ = seconds.get(); }, move || tick.cancel());
/// ```
pub fn interval(ms: u32, f: impl FnMut() + 'static) -> IntervalHandle {
    let cb: Closure<dyn FnMut()> = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
    let id = window()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            clamp_ms(ms),
        )
        .expect("veloasm: window.setInterval failed");
    IntervalHandle {
        id: Some(id),
        _cb: cb,
    }
}

/// Shared state for one [`debounced`] wrapper. Dropping it cancels any
/// still-pending invocation, so a wrapper never outlives its scheduled call.
struct DebounceState<F> {
    pending: Option<i32>,
    cb: Option<Closure<dyn FnMut()>>,
    f: Rc<RefCell<F>>,
}

impl<F> Drop for DebounceState<F> {
    fn drop(&mut self) {
        if let Some(id) = self.pending.take() {
            window().clear_timeout_with_handle(id);
        }
        // Dropping `cb` here is safe: callbacks only run from the JS event
        // loop, never while the wrapper itself is being called.
        self.cb = None;
    }
}

/// Wrap `f` so that calls are coalesced: each call (re)arms a `ms`-delayed
/// timer, and only the last call in a burst actually runs `f` (trailing
/// edge). Typical use: search inputs, auto-scroll on keystrokes, resize
/// handlers.
///
/// ```ignore
/// let mut search = debounced(move || query.set(input.get()), 250);
/// // on:input handler calls `search()` — 10 keystrokes => one run.
/// ```
pub fn debounced<F: FnMut() + 'static>(f: F, ms: u32) -> impl FnMut() {
    let state = Rc::new(RefCell::new(DebounceState {
        pending: None,
        cb: None,
        f: Rc::new(RefCell::new(f)),
    }));
    move || {
        let mut st = state.borrow_mut();
        // Rearm: drop the previous timer and its closure (not running — we
        // are inside the caller's frame, not the callback's).
        if let Some(old) = st.pending.take() {
            window().clear_timeout_with_handle(old);
        }
        st.cb = None;

        let f_cell = Rc::clone(&st.f);
        let cb: Closure<dyn FnMut()> =
            Closure::wrap(Box::new(move || f_cell.borrow_mut()()) as Box<dyn FnMut()>);
        match window().set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            clamp_ms(ms),
        ) {
            Ok(id) => {
                st.pending = Some(id);
                st.cb = Some(cb);
            }
            Err(_) => {
                // No timer available (non-browser host): run inline.
                drop(st);
                (state.borrow().f.borrow_mut())();
            }
        }
    }
}

