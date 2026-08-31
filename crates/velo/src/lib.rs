//! # Velo — the unified Velo web framework
//!
//! Single-crate facade that merges the former `velo_core` (reactivity engine),
//! `velo_dom` (DOM layer), and `velo_router` (client-side router) into one
//! package, plus re-exports the `view!`, `#[component]`, and `routes!`
//! procedural macros from the companion `velo_macro` crate.
//!
//! The only accompanying package is `velo_macro` (a Rust proc-macro crate).
//! Everything else lives here so consumers depend on a single crate.

// =============================================================================
// Re-exports
// =============================================================================

/// The `view!`, `#[component]`, and `routes!` procedural macros (defined in the
/// companion `velo_macro` package).
pub use velo_macro::{component, routes, view};

// =============================================================================
// =============================================================================
// PART 1 — REACTIVITY ENGINE  (formerly `velo_core`)
// =============================================================================
// =============================================================================

use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(usize);

thread_local! {
    static ACTIVE_EFFECT_ID: RefCell<Option<EffectId>> = RefCell::new(None);
    static EFFECT_REGISTRY: RefCell<Vec<Rc<RefCell<Effect>>>> = RefCell::new(Vec::new());
    static EFFECT_COUNTER: RefCell<usize> = RefCell::new(0);

    // --- batching ---
    /// Queue of effects ready to run; drained by `flush_effects`.
    static PENDING_EFFECTS: RefCell<Vec<Rc<RefCell<Effect>>>> = RefCell::new(Vec::new());
    /// Depth of nested `batch()` calls. 0 = no batch active.
    static BATCH_DEPTH: RefCell<usize> = RefCell::new(0);
}

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

pub struct Effect {
    id: EffectId,
    func: Box<dyn FnMut()>,
    /// When true, this effect has been disposed and should no longer run.
    disposed: bool,
    /// Optional cleanup callback run exactly once when the effect is disposed.
    cleanup: Option<Box<dyn FnOnce()>>,
}

impl Effect {
    fn new(id: EffectId, func: Box<dyn FnMut()>) -> Self {
        Self { id, func, disposed: false, cleanup: None }
    }
}

// ---------------------------------------------------------------------------
// Registration / disposal / flushing
// ---------------------------------------------------------------------------

fn register_effect(effect: &Rc<RefCell<Effect>>) {
    EFFECT_REGISTRY.with(|r| r.borrow_mut().push(Rc::clone(effect)));
}

/// Mark an effect as disposed and run its cleanup (if any). Safe to call
/// multiple times — only runs once.  **pub(crate)** — not part of the public API.
pub(crate) fn dispose_effect(effect: &Rc<RefCell<Effect>>) {
    let mut ef = effect.borrow_mut();
    if !ef.disposed {
        ef.disposed = true;
        if let Some(cleanup) = ef.cleanup.take() {
            cleanup();
        }
    }
}

/// Drain all pending effects. Called automatically at the end of `batch()`
/// and also by `create_effect` to ensure any effects queued during the
/// initial run get executed before the function returns.
pub(crate) fn flush_effects() {
    loop {
        let to_run = PENDING_EFFECTS.with(|q| q.borrow_mut().drain(..).collect::<Vec<_>>());
        if to_run.is_empty() {
            break;
        }
        // Dedup by EffectId so an effect queued by multiple signals in one batch
        // runs only once.
        let mut seen: HashSet<EffectId> = HashSet::new();
        let mut deduped: Vec<Rc<RefCell<Effect>>> = Vec::new();
        for effect_rc in to_run {
            let id = effect_rc.borrow().id;
            if !seen.contains(&id) && !effect_rc.borrow().disposed {
                seen.insert(id);
                deduped.push(effect_rc);
            }
        }
        for effect_rc in deduped {
            // Re-register so a second .set() during this run will queue again.
            register_effect(&effect_rc);
            let effect_id = effect_rc.borrow().id;

            let previous_id = ACTIVE_EFFECT_ID.with(|active| active.replace(Some(effect_id)));
            let mut func = std::mem::replace(&mut effect_rc.borrow_mut().func, Box::new(|| {}));
            (func)();
            effect_rc.borrow_mut().func = func;
            ACTIVE_EFFECT_ID.with(|active| active.replace(previous_id));
        }
    }
}

// ---------------------------------------------------------------------------
// SignalInner — the shared reactive cell backing every signal handle.
// ---------------------------------------------------------------------------

pub(crate) struct SignalInner<T> {
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<Rc<RefCell<Effect>>>>>,
}

impl<T> Clone for SignalInner<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

impl<T: Clone + 'static> SignalInner<T> {
    fn new(initial_value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(initial_value)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Read the value and, if an effect is running, subscribe it.
    fn get(&self) -> T {
        ACTIVE_EFFECT_ID.with(|active_id| {
            if let Some(current_id) = *active_id.borrow() {
                EFFECT_REGISTRY.with(|registry| {
                    if let Some(effect_rc) = registry
                        .borrow()
                        .iter()
                        .find(|e| e.borrow().id == current_id && !e.borrow().disposed)
                    {
                        self.mount_effect(&effect_rc);
                    }
                });
            }
        });
        self.value.borrow().clone()
    }

    fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value;
        self.notify();
    }

    fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut *self.value.borrow_mut());
        self.notify();
    }

    /// Notify subscribers by pushing them onto the pending queue.
    ///
    /// If a `batch()` is active (thread-local depth > 0), effects are
    /// accumulated in `PENDING_EFFECTS` and only flushed when the outermost
    /// `batch()` exits. Otherwise they are flushed immediately.
    fn notify(&self) {
        let subs = self.subscribers.borrow().clone();
        let mut added: HashSet<EffectId> = HashSet::new();
        let mut to_queue: Vec<Rc<RefCell<Effect>>> = Vec::new();

        for effect_rc in subs {
            let effect_id = effect_rc.borrow().id;
            if !effect_rc.borrow().disposed && added.insert(effect_id) {
                to_queue.push(effect_rc);
            }
        }

        // Avoid pushing effects that are already in the queue (e.g. when
        // multiple signals in a batch notify the same effect).
        let existing_ids: HashSet<EffectId> = PENDING_EFFECTS.with(|q| {
            q.borrow().iter().map(|e| e.borrow().id).collect()
        });
        for effect_rc in to_queue {
            if !existing_ids.contains(&effect_rc.borrow().id) {
                PENDING_EFFECTS.with(|q| q.borrow_mut().push(effect_rc));
            }
        }

        // If we are not inside a batch, flush immediately so the change is
        // visible synchronously (preserving the current behaviour for code
        // that doesn't use `batch()`).
        if BATCH_DEPTH.with(|d| *d.borrow()) == 0 {
            flush_effects();
        }
    }

    fn mount_effect(&self, effect: &Rc<RefCell<Effect>>) {
        let mut subs = self.subscribers.borrow_mut();
        if !subs.iter().any(|s| s.borrow().id == effect.borrow().id) {
            subs.push(Rc::clone(effect));
        }
    }
}

// ---------------------------------------------------------------------------
// Signal types
// ---------------------------------------------------------------------------

/// Read-only view of a reactive value. Calling `.get()` inside an effect
/// subscribes that effect so it re-runs when the value changes.
pub struct ReadSignal<T> {
    inner: SignalInner<T>,
}

impl<T: Clone + 'static> ReadSignal<T> {
    /// Read the current value (and track the running effect, if any).
    pub fn get(&self) -> T {
        self.inner.get()
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Write-only handle used to update a reactive value.
pub struct WriteSignal<T> {
    inner: SignalInner<T>,
}

impl<T: Clone + 'static> WriteSignal<T> {
    /// Replace the value and notify subscribers.
    pub fn set(&self, new_value: T) {
        self.inner.set(new_value);
    }

    /// Mutate the value in place and notify subscribers.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        self.inner.update(f);
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// A combined read+write handle. Kept for ergonomic single-variable state
/// and for cases (like the router) that need one object to both read and write.
pub struct Signal<T> {
    inner: SignalInner<T>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(initial_value: T) -> Self {
        Self {
            inner: SignalInner::new(initial_value),
        }
    }

    pub fn get(&self) -> T {
        self.inner.get()
    }

    pub fn set(&self, new_value: T) {
        self.inner.set(new_value);
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        self.inner.update(f);
    }

    /// Split into separate read and write handles.
    pub fn split(self) -> (ReadSignal<T>, WriteSignal<T>) {
        (
            ReadSignal {
                inner: self.inner.clone(),
            },
            WriteSignal {
                inner: self.inner.clone(),
            },
        )
    }

    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            inner: self.inner.clone(),
        }
    }

    pub fn write_only(&self) -> WriteSignal<T> {
        WriteSignal {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Effect construction
// ---------------------------------------------------------------------------

/// Create a reactive effect. The closure runs immediately and re-runs whenever
/// any signal it read changes.
///
/// Returns an [`EffectHandle`] that disposes the effect when dropped. Hold the
/// handle for as long as the effect should stay alive; drop it to tear down
/// the effect and run any cleanup closure.
///
/// # Cleaning up
///
/// For cleanup on disposal, use [`create_effect_with_cleanup`]. The handle alone
/// is sufficient for effects that need no teardown — dropping it stops future
/// re-runs and removes the effect from subscriber lists.
pub fn create_effect<F>(func: F) -> EffectHandle
where
    F: FnMut() + 'static,
{
    create_effect_inner(Box::new(func), None)
}

/// Create a reactive effect with a cleanup callback. The cleanup runs exactly
/// once when the effect is disposed — either by dropping the returned
/// [`EffectHandle`] or by calling [`dispose_effect`] manually.
///
/// This is the primary mechanism for tearing down side effects (event listeners,
/// subscriptions, timers) when a component unmounts.
pub fn create_effect_with_cleanup<F, C>(func: F, cleanup: C) -> EffectHandle
where
    F: FnMut() + 'static,
    C: FnOnce() + 'static,
{
    create_effect_inner(Box::new(func), Some(Box::new(cleanup)))
}

/// Internal: construct an effect, register it, run it once, flush queues, return handle.
fn create_effect_inner(
    func: Box<dyn FnMut()>,
    cleanup: Option<Box<dyn FnOnce()>>,
) -> EffectHandle {
    let next_id = EFFECT_COUNTER.with(|counter| {
        let mut c = counter.borrow_mut();
        *c += 1;
        *c
    });
    let id = EffectId(next_id);
    let effect = Effect {
        id,
        func,
        disposed: false,
        cleanup,
    };
    let effect_rc = Rc::new(RefCell::new(effect));
    let effect_rc_clone = Rc::clone(&effect_rc);

    // Register globally
    register_effect(&effect_rc);

    // Initial synchronous execution
    let previous_id = ACTIVE_EFFECT_ID.with(|active| active.replace(Some(id)));
    let mut func = std::mem::replace(&mut effect_rc.borrow_mut().func, Box::new(|| {}));
    (func)();
    effect_rc.borrow_mut().func = func;
    ACTIVE_EFFECT_ID.with(|active| active.replace(previous_id));

    // Flush any effects that were queued during the initial run.
    flush_effects();

    EffectHandle { effect: effect_rc_clone }
}

/// Dispose an effect by its handle. Runs cleanup (if any) and marks the
/// effect as dead so it won't re-run on future signal changes.
///
/// EffectHandles are also `Drop`-based: dropping the handle disposes the
/// effect automatically. Use this function when you need explicit disposal
/// without keeping the handle alive.
pub fn dispose_effect_handle(_handle: EffectHandle) {
    // The drop of _handle does the actual work.
    drop(_handle);
}

/// A handle to a live effect. Dropping it disposes the effect (runs cleanup
/// and stops future re-runs).
#[derive(Clone)]
pub struct EffectHandle {
    effect: Rc<RefCell<Effect>>,
}

impl Drop for EffectHandle {
    fn drop(&mut self) {
        dispose_effect(&self.effect);
    }
}

// ---------------------------------------------------------------------------
// batch() — grouped signal updates
// ---------------------------------------------------------------------------

/// Run `f` inside a batch. Any number of signal `.set()` / `.update()` calls
/// inside `f` (including those triggered transitively by effects) notify
/// subscribers only once, when the batch exits.
///
/// Nested `batch()` calls are a no-op — the outermost batch owns the flush.
/// This means `batch(|| { batch(|| { ... }); })` still flushes only once.
///
/// # Example
///
/// ```ignore
/// let (a, set_a) = create_signal(0);
/// let (b, set_b) = create_signal(0);
/// let count = create_memo(move || a.get() + b.get());
///
/// // Without batch: set_a triggers count, then set_b triggers count again (2 runs).
/// set_a.set(1);
/// set_b.set(2);
///
/// // With batch: count runs once after both sets.
/// batch(|| {
///     set_a.set(1);
///     set_b.set(2);
/// });
/// ```
pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let depth = BATCH_DEPTH.with(|d| {
        let mut cur = d.borrow_mut();
        *cur += 1;
        *cur
    });

    let was_top = depth == 1;
    let result = f();

    if was_top {
        // Drain the accumulated queue exactly once.
        flush_effects();
    }

    result
}

// ---------------------------------------------------------------------------
// SignalVec — a reactive collection with fine-grained subscriber notification.
// ---------------------------------------------------------------------------

/// A reactive `Vec`. Mutations notify subscribers, which can read the current
/// length/items and re-render efficiently (the DOM layer reconciles by key).
pub struct SignalVec<T> {
    inner: SignalInner<Vec<T>>,
}

impl<T: Clone + 'static> SignalVec<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            inner: SignalInner::new(items),
        }
    }

    /// Read the backing slice (and subscribe the running effect, if any).
    pub fn get(&self) -> Vec<T> {
        self.inner.get()
    }

    /// Number of items (subscribes the running effect).
    pub fn len(&self) -> usize {
        self.inner.get().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, item: T) {
        self.inner.update(|v| v.push(item));
    }

    pub fn insert(&self, index: usize, item: T) {
        self.inner.update(|v| v.insert(index, item));
    }

    pub fn remove(&self, index: usize) -> Option<T> {
        let mut out = None;
        self.inner.update(|v| {
            if index < v.len() {
                out = Some(v.remove(index));
            }
        });
        out
    }

    pub fn clear(&self) {
        self.inner.update(|v| v.clear());
    }

    /// Apply a batch of mutations in one notification (prevents N re-renders).
    pub fn with_mut<F: FnOnce(&mut Vec<T>)>(&self, f: F) {
        self.inner.update(f);
    }

    /// Subscribe a callback that receives the (cloned) current items.
    pub fn subscribe<F: FnMut(Vec<T>) + 'static>(&self, mut f: F) -> EffectHandle {
        let inner = self.inner.clone();
        create_effect(move || {
            let items = inner.get();
            f(items);
        })
    }
}

impl<T> Clone for SignalVec<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Context — dependency injection for components (global / shared state).
// ---------------------------------------------------------------------------

use std::any::{Any, TypeId};
use std::collections::HashMap;

thread_local! {
    /// Stack of context maps. Index 0 is the root; components push/pop their own.
    static CONTEXT_STACK: RefCell<Vec<HashMap<TypeId, Rc<dyn Any>>>> =
        RefCell::new(vec![HashMap::new()]);
}

/// Provide a value to the current scope and all descendant components created
/// within `f`. The value is retrievable via [`use_context`].
pub fn provide_context<T: 'static>(value: T) {
    CONTEXT_STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.insert(TypeId::of::<T>(), Rc::new(value) as Rc<dyn Any>);
        }
    });
}

/// Provide a value for the duration of `f`, restoring the previous context
/// afterwards. Use this inside a component body to scope state to subtrees.
pub fn with_context<T: 'static, R>(value: T, f: impl FnOnce() -> R) -> R {
    CONTEXT_STACK.with(|stack| {
        let mut map = stack.borrow().last().cloned().unwrap_or_default();
        map.insert(TypeId::of::<T>(), Rc::new(value) as Rc<dyn Any>);
        stack.borrow_mut().push(map);
    });
    let result = f();
    CONTEXT_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
    result
}

/// Read a value provided by an ancestor [`provide_context`]/[`with_context`].
/// Returns `None` if no matching value is in scope.
pub fn use_context<T: 'static + Clone>() -> Option<T> {
    CONTEXT_STACK.with(|stack| {
        for map in stack.borrow().iter().rev() {
            if let Some(rc) = map.get(&TypeId::of::<T>()) {
                if let Some(v) = rc.downcast_ref::<T>() {
                    return Some(v.clone());
                }
            }
        }
        None
    })
}

// ---------------------------------------------------------------------------
// create_memo, create_signal — defined here so they can be tested.
// ---------------------------------------------------------------------------

/// A derived, cached read-only signal. The closure runs inside an effect, so it
/// automatically re-computes whenever any signal it reads changes.
pub fn create_memo<F, T>(mut f: F) -> Memo<T>
where
    F: FnMut() -> T + 'static,
    T: Clone + 'static,
{
    let init = f();
    let (read, write) = create_signal(init);
    let handle = create_effect({
        let write = write.clone();
        let mut f = f;
        move || {
            let next = f();
            write.set(next);
        }
    });
    Memo {
        read,
        _handle: handle,
    }
}

/// A memo is a `ReadSignal<T>` whose value is recomputed by an effect whenever
/// one of its dependencies changes. Holding the `Memo` keeps the underlying
/// effect alive; dropping it disposes the effect.
#[derive(Clone)]
pub struct Memo<T> {
    read: ReadSignal<T>,
    _handle: EffectHandle,
}

impl<T: Clone + 'static> Deref for Memo<T> {
    type Target = ReadSignal<T>;
    fn deref(&self) -> &Self::Target {
        &self.read
    }
}

/// Create a reactive signal, returning a `(read, write)` handle pair.
pub fn create_signal<T: Clone + 'static>(initial_value: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let inner = SignalInner::new(initial_value);
    (
        ReadSignal {
            inner: inner.clone(),
        },
        WriteSignal { inner },
    )
}

// ---------------------------------------------------------------------------
// RwSignal — zero-clone read+write handle (Leptos-style)
// ---------------------------------------------------------------------------

/// A single handle to reactive state that provides both read (`.get()`) and
/// write (`.set()` / `.update()`) without the caller having to juggle a split
/// `(ReadSignal, WriteSignal)` pair and perform extra clones.
///
/// `RwSignal<T>` is `Clone` — cloning it only shares the underlying
/// `SignalInner`. It is **not** `Copy` because the inner type contains `Rc`;
/// however the user never needs to think about it — the `view!` macro and the
/// generated event handlers clone the handle under the hood, so user code
/// never writes explicit `.clone()` in closures.
pub struct RwSignal<T> {
    inner: SignalInner<T>,
}

impl<T: Clone + 'static> RwSignal<T> {
    /// Create a new reactive value.
    pub fn new(initial_value: T) -> Self {
        Self {
            inner: SignalInner::new(initial_value),
        }
    }

    /// Read the current value (and track the running effect, if any).
    pub fn get(&self) -> T {
        self.inner.get()
    }

    /// Replace the value and notify subscribers.
    pub fn set(&self, new_value: T) {
        self.inner.set(new_value);
    }

    /// Mutate the value in place and notify subscribers.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        self.inner.update(f);
    }
}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: std::fmt::Display + Clone + 'static> std::fmt::Display for RwSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.get(), f)
    }
}

// ---------------------------------------------------------------------------
// Terse factory names
// ---------------------------------------------------------------------------

/// Create a reactive signal, returning a combined `RwSignal<T>` handle.
///
/// This is the ergonomic entry point for state that both the view and event
/// handlers read/write — callers use `r.get()`, `r.set(...)`, `r.update(...)`.
/// No split, no explicit `.clone()` in closures (the macro + event handlers
/// clone the handle when needed).
pub fn signal<T: Clone + 'static>(initial_value: T) -> RwSignal<T> {
    RwSignal::new(initial_value)
}

/// Create a reactive memo (derived, cached read-only signal).
///
/// The closure runs inside an effect, so the memo automatically recomputes when
/// any signal it reads changes. Returns an owned `Memo<T>` that auto-unwraps in
/// `view! { { memo } }`.
pub fn memo<F, T>(f: F) -> Memo<T>
where
    F: FnMut() -> T + 'static,
    T: Clone + 'static,
{
    create_memo(f)
}

/// Create a reactive list (backed by `SignalVec<T>`).
pub fn signal_vec<T: Clone + 'static>(initial: Vec<T>) -> SignalVec<T> {
    SignalVec::new(initial)
}

/// A reactive handle for async data.
#[derive(Clone)]
pub struct Resource<T: Clone + 'static> {
    loading: Signal<bool>,
    value: Signal<Option<T>>,
}

impl<T: Clone + 'static> Resource<T> {
    pub fn loading(&self) -> bool {
        self.loading.get()
    }
    pub fn value(&self) -> Option<T> {
        self.value.get()
    }
}

/// Create a reactive resource from an async future.
pub fn create_resource<F, Fut, T>(fetcher: F) -> Resource<T>
where
    F: Fn() -> Fut + 'static,
    Fut: std::future::Future<Output = T> + 'static,
    T: Clone + 'static,
{
    let loading = Signal::new(true);
    let value = Signal::new(None);

    let loading_c = loading.clone();
    let value_c = value.clone();

    // Use wasm-bindgen-futures to spawn the future
    wasm_bindgen_futures::spawn_local(async move {
        let val = fetcher().await;
        value_c.set(Some(val));
        loading_c.set(false);
    });

    Resource { loading, value }
}

// =============================================================================
// =============================================================================
// PART 2 — DOM  (formerly `velo_dom`)
// =============================================================================
// =============================================================================

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node};

/// Helper to easily access the global window document instance
pub fn document() -> Document {
    web_sys::window()
        .expect("Velo: No global window found. Are you running in a browser environment?")
        .document()
        .expect("Velo: Window should contain a valid document.")
}

pub trait RenderDynamic {
    fn render_dynamic(self) -> DomNode;
}

// Case A: The expression inside the braces returns a single DomNode
impl RenderDynamic for DomNode {
    fn render_dynamic(self) -> DomNode {
        self
    }
}

// Case B: The expression returns a collected Vector of nodes from an iterator
impl RenderDynamic for Vec<DomNode> {
    fn render_dynamic(self) -> DomNode {
        let frag = DomNode::fragment();
        for node in self {
            frag.append(&node);
        }
        frag
    }
}

// Case C: Macro to implement RenderDynamic explicitly for common types.
// This satisfies the compiler completely because Vec<DomNode> is none of these!
macro_rules! impl_render_for_primitives {
    ($($t:ty),*) => {
        $(
            impl RenderDynamic for $t {
                fn render_dynamic(self) -> DomNode {
                    DomNode::text(&format!("{}", self))
                }
            }
        )*
    };
}

// Tell the framework how to dynamically render text, numbers, and booleans inline
impl_render_for_primitives!(
    String, &str, bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

// Any `RenderDynamic` type is a plain (non-signal) view value. `PlainViewValue`
// is a marker implemented only here, never for signals, so the `ViewValue`
// blanket below doesn't overlap with the `Signal`/`ReadSignal` impls.
impl<T: RenderDynamic + 'static> PlainViewValue for T {}

/// Trait powering the `view!` macro's automatic signal unwrapping.
///
/// `view! { { count } }` and `name={ signal }` wrap the expression in
/// `signal_value!(..)`, which calls [`ViewValue::view_value`]. Signal types
/// unwrap to their inner value (and subscribe the running effect); plain
/// `RenderDynamic` values pass through unchanged. Takes `&self` so the source
/// handle is borrowed, not moved (handles may be reused in other closures).
pub trait ViewValue {
    type Out;
    fn view_value(&self) -> Self::Out;
}

impl<T: Clone + 'static> ViewValue for ReadSignal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for Signal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for Memo<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for RwSignal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

// Marker trait so plain RenderDynamic values can get a ViewValue blanket
// impl without overlapping the `Signal`/`ReadSignal` impls above.
pub trait PlainViewValue {}

// Gated on `PlainViewValue` so it never overlaps with the `Signal`/`ReadSignal`
// impls above (those types intentionally do not implement `PlainViewValue`).
impl<T: PlainViewValue + Clone + 'static> ViewValue for T {
    type Out = T;
    fn view_value(&self) -> T {
        self.clone()
    }
}

/// Helper used by the `view!` macro: read a reactive value (or pass through a
/// plain value) inside the current effect so it becomes a tracking dependency.
#[macro_export]
macro_rules! signal_value {
    ($expr:expr) => {{
        $crate::ViewValue::view_value(&$expr)
    }};
}

/// A wrapper around a real native browser DOM element
#[derive(Clone)]
pub struct DomNode {
    pub raw_node: Node,
}

impl DomNode {
    /// Creates a standard HTML element container (e.g., "div", "button", "h1")
    pub fn element(tag: &str) -> Self {
        let el = document()
            .create_element(tag)
            .expect("Velo: Failed to create DOM element tag");
        Self {
            raw_node: el.into(),
        }
    }

    /// Creates an empty DocumentFragment node to act as a zero-wrapper container for siblings.
    /// When appended to a parent, the browser automatically unpacks its children directly.
    pub fn fragment() -> Self {
        let frag = document().create_document_fragment();
        Self {
            raw_node: frag.into(),
        }
    }

    /// Creates a static, un-changing text node
    pub fn text(content: &str) -> Self {
        let txt = document().create_text_node(content);
        Self {
            raw_node: txt.into(),
        }
    }

    /// Sets the text content of this node directly, replacing any existing children.
    pub fn set_text(&self, content: &str) {
        self.raw_node.set_text_content(Some(content));
    }

    /// Creates a modern, fine-grained reactive text node.
    /// The node binds itself to a reactive Signal closure and updates surgically.
    pub fn reactive_text<F>(mut f: F) -> Self
    where
        F: FnMut() -> String + 'static,
    {
        let txt_node = document().create_text_node("");
        let current_node = txt_node.clone();

        // Establish the tracking wrapper loop
        create_effect(move || {
            let evaluated_string = f();
            current_node.set_node_value(Some(&evaluated_string));
        });

        Self {
            raw_node: txt_node.into(),
        }
    }

    /// Appends a child DomNode into this node's layout structure
    pub fn append(&self, child: &DomNode) {
        self.raw_node
            .append_child(&child.raw_node)
            .expect("Velo: Failed to append child node layout hook");
    }

    /// Binds an attribute (like "class" or "id") directly to a reactive Signal
    pub fn reactive_attribute<F>(&self, name: &str, mut f: F)
    where
        F: FnMut() -> String + 'static,
    {
        // We cast the generic Node back to an Element to mutate attributes safely
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only bind attributes to element nodes");
        let attr_name = name.to_string();

        create_effect(move || {
            let value = f();
            el.set_attribute(&attr_name, &value)
                .expect("Velo: Failed to update element node attribute");
        });
    }

    /// Attaches an event listener directly to the browser element wrapper
    pub fn on<F>(&self, event_name: &str, mut handler: F)
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            handler(e);
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.raw_node
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
            .expect("Velo: Failed to attach event listener callback");

        // Prevent Rust from cleaning up the closure allocation context immediately
        closure.forget();
    }

    /// Explicitly handles components, sub-views, or existing DomNodes passed into braces
    pub fn from_node(node: DomNode) -> Self {
        node
    }

    /// Explicitly handles dynamic closures passed into braces for surgical text updates
    pub fn from_closure<F>(mut f: F) -> Self
    where
        F: FnMut() -> String + 'static,
    {
        Self::reactive_text(move || f())
    }

    /// Toggles a single class name on/off reactively based on a boolean signal.
    pub fn toggle_class<F>(&self, class_name: &str, mut is_on: F)
    where
        F: FnMut() -> bool + 'static,
    {
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only toggle classes on element nodes");
        let class_name = class_name.to_string();

        create_effect(move || {
            let on = is_on();
            if on {
                let _ = el.class_list().add_1(&class_name);
            } else {
                let _ = el.class_list().remove_1(&class_name);
            }
        });
    }

    /// Binds a CSS inline style property reactively to a string value. Multiple
    /// `reactive_style` calls on the same element are merged (each keeps its own
    /// property) by accumulating into the element's `style` attribute.
    pub fn reactive_style<F>(&self, property: &str, mut f: F)
    where
        F: FnMut() -> String + 'static,
    {
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only bind styles to element nodes");
        let property = property.to_string();

        // Accumulate inline style properties so concurrent bindings don't clobber.
        let styles: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
        let styles_c = std::rc::Rc::clone(&styles);

        create_effect(move || {
            let value = f();
            styles_c.borrow_mut().insert(property.clone(), value);

            let css: String = styles_c
                .borrow()
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = el.set_attribute("style", &css);
        });
    }

    /// Mounts a keyed, fine-grained reactive list. On each change the reconciler
    /// inserts/removes/moves real DOM nodes to match the new keyed items, instead
    /// of blowing away the whole subtree. `key` extracts the stable key; `render`
    /// builds the `DomNode` for one item.
    pub fn render_signal_vec<T, K, FKey, FRender>(
        &self,
        list: &SignalVec<T>,
        key: FKey,
        render: FRender,
    ) where
        T: Clone + 'static,
        K: Eq + std::hash::Hash + Clone + 'static,
        FKey: Fn(&T) -> K + 'static,
        FRender: Fn(&T) -> DomNode + 'static + Clone,
    {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let container: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only render a list into an element node");

        // Node map keyed by the item's stable key, plus an ordered list of keys
        // so we can detect moves/restores cheaply.
        let nodes: Rc<RefCell<HashMap<K, DomNode>>> = Rc::new(RefCell::new(HashMap::new()));
        let order: Rc<RefCell<Vec<K>>> = Rc::new(RefCell::new(Vec::new()));

        let list = list.clone();
        let nodes_c = Rc::clone(&nodes);
        let order_c = Rc::clone(&order);

        create_effect(move || {
            let items: Vec<T> = list.get();
            let new_keys: Vec<K> = items.iter().map(|it| key(it)).collect();

            // Remove nodes whose key disappeared.
            let live: std::collections::HashSet<K> = new_keys.iter().cloned().collect();
            let stale: Vec<K> = order_c
                .borrow()
                .iter()
                .filter(|k| !live.contains(k))
                .cloned()
                .collect();
            for k in stale {
                if let Some(node) = nodes_c.borrow_mut().remove(&k) {
                    let _ = container.remove_child(&node.raw_node);
                }
            }

            // Build a lookup of new items by key for quick render.
            let mut by_key: HashMap<K, DomNode> = HashMap::new();
            for it in &items {
                let k = key(it);
                let node = render(it);
                by_key.insert(k, node);
            }

            // Reconcile against previous order: insert/move each item before the
            // node that precedes it in the previous layout (or append at end).
            let prev = order_c.borrow().clone();
            for (idx, k) in new_keys.iter().enumerate() {
                let node = match nodes_c.borrow_mut().get(k) {
                    Some(existing) => existing.clone(),
                    None => {
                        let n = by_key.remove(k).expect("rendered node for key");
                        nodes_c.borrow_mut().insert(k.clone(), n.clone());
                        n
                    }
                };

                let reference = if idx + 1 < new_keys.len() {
                    nodes_c
                        .borrow()
                        .get(&new_keys[idx + 1])
                        .map(|n| n.raw_node.clone())
                } else {
                    None
                };

                // Only re-insert if its position actually changed.
                let needs_move = prev.get(idx) != Some(k);
                if needs_move {
                    let _ = container.insert_before(&node.raw_node, reference.as_ref());
                }
            }

            *order_c.borrow_mut() = new_keys;
        });
    }

    /// Accepts a closure from the macro, evaluates it inside an effect loop,
    /// and resolves whether it's rendering a component or text dynamically!
    pub fn render_expression<F, R>(mut f: F) -> Self
    where
        F: FnMut() -> R + 'static,
        R: RenderDynamic + 'static,
    {
        // Use a fragment as the container for dynamic expressions.
        // Fragments are transparent containers that unpack into their parent.
        let container = DomNode::fragment();
        let container_raw = container.raw_node.clone();

        let mut f_clone = move || f();

        create_effect(move || {
            let val: R = f_clone();
            let resolved_node = val.render_dynamic();

            // Clear the existing content of the fragment.
            while let Some(child) = container_raw.first_child() {
                container_raw.remove_child(&child).unwrap();
            }

            container_raw
                .append_child(&resolved_node.raw_node)
                .expect("Velo: Failed to append dynamic expression variant");
        });

        container
    }
}

/// Mounts the root framework application element directly to a target DOM
/// container ID.
///
/// **Deprecated:** prefer [`mount`] (body) or [`mount_at`] (explicit node).
/// `mount_to_id` appends as a child (creating a wrapper) and cannot be
/// unmounted via a returned handle.
#[deprecated(note = "prefer mount()/mount_at(); see Velo docs §8")]
pub fn mount_to_id(id: &str, root_node: DomNode) {
    let container = document()
        .get_element_by_id(id)
        .expect("Velo: Mount targets specified container element ID could not be located");

    container
        .append_child(&root_node.raw_node)
        .expect("Velo: Failed to mount root node allocation to layout tree container target");
}

// ---------------------------------------------------------------------------
// Modern Mounting API  (§8): mount(), mount_at(), RootHandle
// ---------------------------------------------------------------------------

/// A handle to a mounted Velo root. Dropping it unmounts the app from the
/// DOM (removes the root node from its parent). Calling `.unmount()` does
/// the same explicitly and is idempotent.
///
/// Returned by [`mount`] and [`mount_at`] so the caller can tear down the
/// entire app (useful for testing, remounting, and hot-reload teardown).
#[derive(Clone)]
pub struct RootHandle {
    root: DomNode,
}

impl RootHandle {
    /// Remove the root node(s) from the DOM. Safe to call multiple times.
    pub fn unmount(self) {
        if let Some(parent) = self.root.raw_node.parent_node() {
            let _ = parent.remove_child(&self.root.raw_node);
        }
    }
}

impl Drop for RootHandle {
    fn drop(&mut self) {
        // Dispose any root-level effects here once effect-to-node tracking is
        // in place. For now the DomNode Drop removes the node itself.
        if let Some(parent) = self.root.raw_node.parent_node() {
            let _ = parent.remove_child(&self.root.raw_node);
        }
    }
}

/// Convenience: mount a `DomNode` tree into `document.body()` as a fragment
/// root (no wrapper element). Returns a `RootHandle` that can be dropped or
/// `.unmount()`ed to tear the app down.
///
/// ```ignore
/// let app = view! { <div class="page">...</div> };
/// let handle = velo::mount(app);
/// // later: handle.unmount();
/// ```
pub fn mount(root: DomNode) -> RootHandle {
    let body = document()
        .body()
        .expect("Velo: document has no body — are you running in a browser?");
    mount_at(&body, root)
}

/// Mount a `DomNode` tree into a specific DOM `Node` as a fragment root.
/// The root is **appended** into the target as a child (fragment roots
/// unpack automatically, so there is no wrapper element).
///
/// Returns a `RootHandle` for explicit unmount / Drop-based teardown.
///
/// ```ignore
/// let app = view! { <div class="page">...</div> };
/// let div = document().get_element_by_id("app").unwrap();
/// let handle = velo::mount_at(&div, app);
/// ```
pub fn mount_at(target: &web_sys::Node, root: DomNode) -> RootHandle {
    target
        .append_child(&root.raw_node)
        .expect("Velo: Failed to mount root node into target");
    RootHandle { root }
}

/// Mounts the root framework application element directly to a target DOM
/// container ID.
///
/// **Deprecated:** prefer [`mount`] (body) or [`mount_at`] (explicit node).
/// `mount_to_id` appends as a child (creating a wrapper) and cannot be
/// unmounted via a returned handle.
#[deprecated(note = "prefer mount()/mount_at(); see Velo docs §8")]
pub fn mount_to_id_deprecated(id: &str, root_node: DomNode) {
    let container = document()
        .get_element_by_id(id)
        .expect("Velo: Mount targets specified container element ID could not be located");

    container
        .append_child(&root_node.raw_node)
        .expect("Velo: Failed to mount root node allocation to layout tree container target");
}

pub fn Show(children: Vec<DomNode>, when: bool, fallback: Option<DomNode>) -> DomNode {
    if when {
        let frag = DomNode::fragment();
        for child in children {
            frag.append(&child);
        }
        frag
    } else {
        fallback.unwrap_or_else(|| DomNode::text(""))
    }
}

// =============================================================================
// =============================================================================
// PART 3 — ROUTER  (formerly `velo_router`)
// =============================================================================
// =============================================================================

thread_local! {
    // Global tracking signal for the current URL path string
    pub static CURRENT_PATH: Signal<String> = Signal::new(
        web_sys::window()
            .expect("Velo Router: No window found")
            .location()
            .pathname()
            .expect("Velo Router: Failed to read pathname")
    );
    pub static CURRENT_QUERY: Signal<HashMap<String, String>> =
        Signal::new(parse_query_string(&web_sys::window().expect("Velo Router: No window found").location().search().unwrap_or_default()));
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let s = query.strip_prefix('?').unwrap_or(query);
    if s.is_empty() {
        return map;
    }
    for pair in s.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(url_decode(k), url_decode(v));
        } else {
            map.insert(url_decode(pair), String::new());
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s).decode_utf8_lossy().to_string()
}

/// Programmatically updates the browser URL and alerts the active Route Signal
pub fn navigate_to(path: &str) {
    let window = web_sys::window().expect("Velo Router: No window found");
    let history = window
        .history()
        .expect("Velo Router: Accessing history failed");

    // Update browser history state without refreshing the page
    history
        .push_state_with_url(&JsValue::NULL, "", Some(path))
        .expect("Velo Router: push_state failed");

    // Inform our fine-grained reactive loop about the location update
    CURRENT_PATH.with(|path_signal| {
        path_signal.set(path.to_string());
    });

    let query = parse_query_string(&window.location().search().unwrap_or_default());
    CURRENT_QUERY.with(|q| q.set(query));
}

/// Initializes global browser listeners to intercept browser back/forward buttons
pub fn init_router_listeners() {
    let window = web_sys::window().expect("Velo Router: No window found");

    let on_popstate = Closure::wrap(Box::new(move |_e: web_sys::PopStateEvent| {
        let current_path_str = web_sys::window().unwrap().location().pathname().unwrap();
        let query = parse_query_string(&web_sys::window().unwrap().location().search().unwrap_or_default());

        CURRENT_PATH.with(|path_signal| {
            path_signal.set(current_path_str);
        });
        CURRENT_QUERY.with(|q| q.set(query));
    }) as Box<dyn FnMut(web_sys::PopStateEvent)>);

    window
        .add_event_listener_with_callback("popstate", on_popstate.as_ref().unchecked_ref())
        .expect("Velo Router: Failed to bind popstate listener");

    on_popstate.forget();
}

fn match_route_patterns(template: &str, incoming_url: &str) -> Option<HashMap<String, String>> {
    // Handle the absolute catch-all wildcard rule immediately
    if template == "/**" {
        return Some(HashMap::new());
    }

    // Split paths into clean structural token segments, dropping empty padding
    let t_segments: Vec<&str> = template.split('/').filter(|s| !s.is_empty()).collect();
    let u_segments: Vec<&str> = incoming_url.split('/').filter(|s| !s.is_empty()).collect();

    // Check for standard wildcard endings at a template level
    let has_wildcard = template.ends_with("/**");

    if !has_wildcard && t_segments.len() != u_segments.len() {
        return None;
    }

    let mut extracted_params = HashMap::new();

    for (i, t_seg) in t_segments.iter().enumerate() {
        if *t_seg == "**" {
            // Found a trailing wildcard! Everything remaining matches perfectly.
            return Some(extracted_params);
        }

        if t_seg.starts_with(':') {
            // 🚀 FOUND A PARAMETER METRIC SEGMENT KEY!
            if let Some(u_seg) = u_segments.get(i) {
                let key = t_seg[1..].to_string(); // Strip the leading colon ":"
                let value = u_seg.to_string();
                extracted_params.insert(key, value);
            } else {
                return None;
            }
        } else {
            // Static segment string match checkpoint (e.g., "dashboard" == "dashboard")
            match u_segments.get(i) {
                Some(u_seg) if u_seg == t_seg => continue,
                _ => return None,
            }
        }
    }

    Some(extracted_params)
}

thread_local! {
    // Stores the active route parameters globally
    static ACTIVE_PARAMS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

pub struct Route {
    pub path: &'static str,
    pub component: fn() -> DomNode,
}

/// Router structural component. Listens to path changes and morphs the core view.
pub struct FRouter;

impl FRouter {
    pub fn new<F>(mut routes_matcher: F) -> DomNode
    where
        F: FnMut(&str) -> DomNode + 'static,
    {
        // Setup global event interception right away
        init_router_listeners();

        // Create a persistent placeholder element where pages dynamically swap out
        let view_wrapper = DomNode::element("div");
        view_wrapper.reactive_attribute("class", || "velo-router-viewport".to_string());

        let current_child: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));

        let wrapper_raw = view_wrapper.raw_node.clone();
        let child_tracker = Rc::clone(&current_child);

        // This effect executes SURGICALLY only when the URL changes
        create_effect(move || {
            let path = CURRENT_PATH.with(|p| p.get());

            // Clean up previous view node from browser memory layout tree if it exists
            if let Some(old_node) = child_tracker.borrow().as_ref() {
                let _ = wrapper_raw.remove_child(&old_node.raw_node);
            }

            // Build the new page node matched via the developer's closure
            let new_page_node = routes_matcher(&path);

            wrapper_raw
                .append_child(&new_page_node.raw_node)
                .expect("Velo Router: Failed to append target route content");

            // Store reference to clean up on the next navigation cycle
            *child_tracker.borrow_mut() = Some(new_page_node);
        });

        view_wrapper
    }

    /// Retrieve a route parameter by key from anywhere in the application
    pub fn param(key: &str) -> Option<String> {
        ACTIVE_PARAMS.with(|p| p.borrow().get(key).cloned())
    }

    /// Helper to grab all parameters if needed
    pub fn params() -> HashMap<String, String> {
        ACTIVE_PARAMS.with(|p| p.borrow().clone())
    }

    /// Retrieve a typed route parameter with automatic parsing.
    /// Returns `None` if the parameter is missing or fails to parse.
    pub fn use_param<T: std::str::FromStr>(key: &str) -> Option<T> {
        ACTIVE_PARAMS.with(|p| p.borrow().get(key).and_then(|v| v.parse::<T>().ok()))
    }

    /// Retrieve a typed query parameter with automatic parsing.
    /// Returns `None` if the parameter is missing or fails to parse.
    pub fn use_query<T: std::str::FromStr>(key: &str) -> Option<T> {
        CURRENT_QUERY.with(|q| q.get().get(key).and_then(|v| v.parse::<T>().ok()))
    }

    /// Get the current path string from the router.
    pub fn use_route() -> String {
        CURRENT_PATH.with(|p| p.get())
    }
}

// =============================================================================
// Ergonomic Router component for clean macro nesting: <Router routes={...} />
// =============================================================================
#[allow(non_snake_case)]
pub struct RouterProps {
    pub routes: Vec<Route>,
}

#[allow(non_snake_case)]
pub fn Router(props: RouterProps) -> DomNode {
    let RouterProps { routes } = props;

    static mut LISTENERS_INITIALIZED: bool = false;
    unsafe {
        if !LISTENERS_INITIALIZED {
            init_router_listeners();
            LISTENERS_INITIALIZED = true;
        }
    }

    let view_wrapper = DomNode::element("div");
    view_wrapper.reactive_attribute("class", || "velo-router-viewport".to_string());

    let current_child: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));
    let wrapper_raw = view_wrapper.raw_node.clone();
    let child_tracker = Rc::clone(&current_child);

    create_effect(move || {
        let current_path = CURRENT_PATH.with(|p| p.get());

        if let Some(old_node) = child_tracker.borrow().as_ref() {
            let _ = wrapper_raw.remove_child(&old_node.raw_node);
        }

        let mut params_payload = HashMap::new();

        let matched_route = routes.iter().find(|r| {
            if let Some(parsed_map) = match_route_patterns(r.path, &current_path) {
                params_payload = parsed_map;
                true
            } else {
                false
            }
        });

        // --- INTERCEPTION LAYER ---
        // Store the parsed parameters globally BEFORE calling the component factory
        ACTIVE_PARAMS.with(|p| {
            *p.borrow_mut() = params_payload;
        });

        let matched_component = match matched_route {
            // Your component functions no longer need to accept parameters!
            Some(route) => (route.component)(),
            None => {
                let fallback = DomNode::element("h1");
                fallback.append(&DomNode::text("404 - Page Not Found"));
                fallback
            }
        };

        wrapper_raw
            .append_child(&matched_component.raw_node)
            .expect("Velo Router: Failed to append target route content");

        *child_tracker.borrow_mut() = Some(matched_component);
    });

    view_wrapper
}

/// Props for [`Link`]: `<Link to="..." label="..." />`.
#[allow(non_snake_case)]
pub struct LinkProps {
    pub to: &'static str,
    pub label: &'static str,
}

/// Ergonomic Link component that allows clean macro nesting: <Link to="...">Children</Link>
#[allow(non_snake_case)]
pub fn Link(props: LinkProps) -> DomNode {
    let LinkProps { to, label } = props;
    let anchor = DomNode::element("a");
    anchor.reactive_attribute("href", move || to.to_string());

    let text_content = DomNode::text(label);
    anchor.append(&text_content);

    anchor.on("click", move |event| {
        event.prevent_default();
        navigate_to(to);
    });

    anchor
}

// =============================================================================
// =============================================================================
// PRELUDE + LAYOUT RE-EXPORTS
// =============================================================================
// =============================================================================

/// The Unified Framework Prelude.
///
/// Bringing this into scope via `use velo::prelude::*;` fully seeds your
/// application files with reactivity primitives, DOM nodes, components,
/// routers, and the view! macro.
pub mod prelude {
    // Re-export core reactive primitives
    pub use crate::{
        batch,
        create_effect,
        create_effect_with_cleanup,
        create_memo,
        create_resource,
        create_signal,
        memo,
        provide_context,
        signal,
        signal_vec,
        use_context,
        with_context,
        ReadSignal,
        Resource,
        RwSignal,
        Signal,
        SignalVec,
        WriteSignal,
    };

    // Re-export the signal-unwrapping machinery used by the view! macro
    #[doc(hidden)]
    pub use crate::PlainViewValue;
    pub use crate::{signal_value, ViewValue};

    // Re-export primary DOM manipulation types and helpers
    pub use crate::{
        document, mount, mount_at, mount_to_id, DomNode, RenderDynamic, RootHandle,
    };

    // Re-export router structures
    pub use crate::{FRouter, Link, LinkProps, Route, Router, RouterProps};

    // Re-export the view! + #[component] + routes! procedural macros
    pub use crate::{component, routes, view};

    // Re-export the built-in control-flow component `Show`.
    pub use crate::Show;
}
