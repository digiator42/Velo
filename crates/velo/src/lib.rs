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

/// The `view!`, `#[component]`, `routes!`, and `#[route]` procedural macros
/// (defined in the companion `velo_macro` package).
pub use velo_macro::{app, component, error, layout, loading, not_found, page, route, routes, route_path, view};

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
    // Take the cleanup callback out first, then release the borrow BEFORE
    // invoking it. Running the cleanup while still holding `borrow_mut()` on
    // the effect panics if the cleanup reads a signal (which re-enters the
    // reactor via `mount_effect` -> `effect.borrow()` on this same effect).
    let cleanup = {
        let mut ef = effect.borrow_mut();
        if !ef.disposed {
            ef.disposed = true;
            ef.cleanup.take()
        } else {
            None
        }
    };
    if let Some(cleanup) = cleanup {
        cleanup();
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
// SignalEngine / SignalInner — the shared reactive cell behind every signal handle.
// ---------------------------------------------------------------------------

/// The heap-allocated reactive cell behind every signal handle: the current
/// value plus the list of effects currently subscribed to it.
pub(crate) struct SignalEngine<T> {
    value: RefCell<T>,
    subscribers: RefCell<Vec<Rc<RefCell<Effect>>>>,
}

/// A `Copy` shared handle to a [`SignalEngine`].
///
/// `Copy` and `Drop` are mutually exclusive in Rust, so the engine can never be
/// reference-counted back down to nothing. It is therefore deliberately
/// retained for the lifetime of the app — exactly like the effects that
/// `reactive_text` / `reactive_switch` `mem::forget`. That is the price of
/// zero-`.clone()` ergonomics: `move ||` and `async move { .. }` handlers
/// capture the handle by value over and over, and `Copy` lets them share it
/// without cloning.
pub(crate) struct SignalInner<T> {
    ptr: *const SignalEngine<T>,
}

impl<T> SignalInner<T> {
    fn new(initial_value: T) -> Self {
        let engine = Box::new(SignalEngine {
            value: RefCell::new(initial_value),
            subscribers: RefCell::new(Vec::new()),
        });
        // Intentionally leaked: retained for the app's lifetime, see struct docs.
        Self {
            ptr: Box::into_raw(engine),
        }
    }

    /// Access the engine the handle points into.
    fn engine(&self) -> &SignalEngine<T> {
        // `ptr` is set once by `new` from a `Box` that is never freed, so it
        // always points at a live `SignalEngine` for the lifetime of the handle.
        unsafe { &*self.ptr }
    }
}

impl<T> Clone for SignalInner<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SignalInner<T> {}

impl<T: Clone + 'static> SignalInner<T> {
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
        self.engine().value.borrow().clone()
    }

    fn set(&self, new_value: T) {
        *self.engine().value.borrow_mut() = new_value;
        self.notify();
    }

    fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut *self.engine().value.borrow_mut());
        self.notify();
    }

    /// Notify subscribers by pushing them onto the pending queue.
    ///
    /// If a `batch()` is active (thread-local depth > 0), effects are
    /// accumulated in `PENDING_EFFECTS` and only flushed when the outermost
    /// `batch()` exits. Otherwise they are flushed immediately.
    fn notify(&self) {
        let subs = self.engine().subscribers.borrow().clone();
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
        let mut subs = self.engine().subscribers.borrow_mut();
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

// Read-only handles are thin `Copy` shared handles over `SignalInner` (a
// pointer into a leaked `SignalEngine`). This lets them move freely into
// `move ||` closures and `async () => { .. }` futures without `.clone()`
// boilerplate.
impl<T> Copy for ReadSignal<T> {}

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

impl<T> Copy for WriteSignal<T> {}

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

impl<T> Copy for Signal<T> {}

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
/// `RwSignal<T>` is `Copy` — it is a thin pointer into a leaked
/// `SignalEngine`, so it can be captured freely by `move ||` and
/// `async move` closures without `.clone()` boilerplate (see `SignalInner`
/// for why the engine is retained).
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

impl<T> Copy for RwSignal<T> {}

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

// ---------------------------------------------------------------------------
// Ergonomic macros — same underlying functions, friendlier call sites.
// ---------------------------------------------------------------------------
//
// These are pure sugar: each one expands to exactly the function call it
// names below. Nothing new happens at runtime; they exist so common state
// setup reads as a short, memorable verb instead of a `create_*`/`*_context`
// function name.

/// `signal!(value)` — shorthand for [`signal`], creating a combined
/// read+write `RwSignal<T>`.
///
/// ```ignore
/// let count = signal!(0);
/// count.set(1);
/// ```
///
/// The split `(ReadSignal, WriteSignal)` pair is still available by calling
/// [`create_signal`] directly for call sites that specifically want it.
#[macro_export]
macro_rules! signal {
    ($value:expr) => {
        $crate::signal($value)
    };
}

/// `provide!(value)` — shorthand for [`provide_context`].
///
/// ```ignore
/// provide!(AppConfig { theme: "dark" });
/// ```
#[macro_export]
macro_rules! provide {
    ($value:expr) => {
        $crate::provide_context($value)
    };
}

/// `context!()` — shorthand for [`use_context`] with the target type
/// inferred from how the result is used. `context!(Type)` spells the type
/// out explicitly (equivalent to `use_context::<Type>()`) when inference
/// can't find it on its own.
///
/// ```ignore
/// let config: Option<AppConfig> = context!();
/// let config = context!(AppConfig);
/// ```
#[macro_export]
macro_rules! context {
    () => {
        $crate::use_context()
    };
    ($ty:ty) => {
        $crate::use_context::<$ty>()
    };
}

/// `effect!(closure)` — shorthand for [`create_effect`].
/// `effect!(closure, cleanup)` — shorthand for [`create_effect_with_cleanup`],
/// running `cleanup` exactly once when the effect is disposed.
///
/// ```ignore
/// effect!(move || log(count.get()));
/// effect!(
///     move || attach_listener(),
///     move || detach_listener(),
/// );
/// ```
#[macro_export]
macro_rules! effect {
    ($f:expr) => {
        $crate::create_effect($f)
    };
    ($f:expr, $cleanup:expr) => {
        $crate::create_effect_with_cleanup($f, $cleanup)
    };
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

/// Sleep asynchronously for `ms` milliseconds.
///
/// A timer-backed future so `async () => { .. }` event handlers can `.await`
/// something real (debounces, "saved" flashes, staged multi-step updates)
/// without hand-writing `spawn_local` or `setTimeout` wiring. Implemented as a
/// `js_sys::Promise` resolved through a `Window.setTimeout`.
pub async fn sleep(ms: u32) {
    let promise = js_sys::Promise::new(
        &mut |resolve: js_sys::Function, _reject: js_sys::Function| {
            let window = web_sys::window().expect(
                "Velo: No global window found. Are you running in a browser environment?",
            );
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                ms as i32,
            );
        },
    );
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

// =============================================================================
// `velo::fetch` — ergonomic JS `fetch` sugar
//
// Wraps `window.fetch` behind an awaitable future (mirroring the JS
// `fetch(url).then(r => r.text())` feel, but in Rust), so developers can
// `.await` it directly inside Velo's `async () => {}` event/reactive handlers.
// =============================================================================

/// A `window.fetch` response, mirroring the JS `Response` ergonomics.
///
/// Obtained via [`fetch`]. Call `.text()` or `.json()` to read the body.
#[derive(Clone)]
pub struct VeloResponse {
    inner: web_sys::Response,
}

impl VeloResponse {
    fn from_response(resp: web_sys::Response) -> Self {
        Self { inner: resp }
    }

    /// HTTP status code (e.g. `200`, `404`).
    pub fn status(&self) -> u16 {
        self.inner.status()
    }

    /// `true` for statuses in the 200–299 range.
    pub fn ok(&self) -> bool {
        self.inner.ok()
    }

    /// The status text (e.g. `"OK"` / `"Not Found"`).
    pub fn status_text(&self) -> String {
        self.inner.status_text()
    }

    /// The final URL the request was served from (after any redirects).
    pub fn url(&self) -> String {
        self.inner.url()
    }

    /// Access a response header by name.
    pub fn header(&self, name: &str) -> Option<String> {
        self.inner
            .headers()
            .get(name)
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
    }

    /// Read the entire body as text.
    pub async fn text(self) -> Result<String, JsValue> {
        let text_promise = self.inner.text()?;
        let text = wasm_bindgen_futures::JsFuture::from(text_promise).await?;
        Ok(text.as_string().ok_or_else(|| {
            JsValue::from_str("Velo: response.body.text() returned a non-string value")
        })?)
    }

    /// Read the body as JSON.
    ///
    /// Returns the raw `js_sys::JsValue` holding the parsed value. For typed
    /// decoding use the serde-backed [`fetch_json`] helper (requires the
    /// `json` feature) instead.
    pub async fn json(self) -> Result<JsValue, JsValue> {
        let json_promise = self.inner.json()?;
        wasm_bindgen_futures::JsFuture::from(json_promise).await
    }
}

/// An error raised while fetching, reading, or decoding a [`VeloResponse`].
#[derive(Debug, Clone)]
pub enum FetchError {
    /// `window.fetch` itself rejected (network / CORS / security error).
    Network(String),
    /// The response returned a non-2xx status.
    Status { code: u16, reason: String },
    /// The response body could not be decoded as JSON.
    Decode(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Network(msg) => write!(f, "fetch network error: {msg}"),
            FetchError::Status { code, reason } => {
                write!(f, "fetch returned {code} {reason}")
            }
            FetchError::Decode(msg) => write!(f, "fetch JSON decode error: {msg}"),
        }
    }
}

impl std::error::Error for FetchError {}

impl From<JsValue> for FetchError {
    fn from(v: JsValue) -> Self {
        FetchError::Network(
            v.as_string()
                .unwrap_or_else(|| "unknown JS fetch error".to_string()),
        )
    }
}

/// Awaitable `window.fetch` wrapper.
///
/// Returns a [`VeloResponse`] whose body can be read with `.text()` or
/// `.json()`. Can be `.await`ed directly inside Velo's `async () => {}`
/// handlers; pairs with [`fetch_json`] for typed JSON.
///
/// ```rust,ignore
/// on:click={ async () => {
///     let resp = velo::fetch("/api/health").await.unwrap();
///     if resp.ok() {
///         log!("up: {}", resp.status());
///     }
/// } }
/// ```
pub async fn fetch(url: &str) -> Result<VeloResponse, JsValue> {
    let window = web_sys::window()
        .ok_or_else(|| JsValue::from_str("Velo: No global window found for fetch"))?;
    let promise = window.fetch_with_str(url);
    let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
    let response: web_sys::Response =
        wasm_bindgen::JsCast::dyn_into(response).map_err(|_| {
            JsValue::from_str("Velo: fetch resolved to a non-Response object")
        })?;
    Ok(VeloResponse::from_response(response))
}

/// Fetch a JSON resource and decode it into typed data — the Rust analogue of
/// `await (await fetch(url)).json()`. Pairs with Velo's `async () => {}`
/// handlers for a seamless Next.js-like data-fetch feel:
///
/// ```rust,ignore
/// #[derive(serde::Deserialize)]
/// struct User { name: String, age: u8 }
///
/// let data = create_resource(|| async {
///     velo::fetch_json::<User>("/api/users/1").await.unwrap()
/// });
/// ```
#[cfg(feature = "json")]
pub async fn fetch_json<T>(url: &str) -> Result<T, FetchError>
where
    T: serde::de::DeserializeOwned,
{
    let window = web_sys::window()
        .ok_or_else(|| FetchError::Network("No window for fetch".into()))?;
    let promise = window.fetch_with_str(url);
    let response = wasm_bindgen_futures::JsFuture::from(promise).await?;
    let response: web_sys::Response =
        wasm_bindgen::JsCast::dyn_into(response).map_err(|_| {
            FetchError::Decode("fetch resolved to a non-Response".into())
        })?;

    if !response.ok() {
        return Err(FetchError::Status {
            code: response.status(),
            reason: response.status_text(),
        });
    }

    let json_promise = response.json()?;
    let json: JsValue = wasm_bindgen_futures::JsFuture::from(json_promise).await?;
    json.into_serde().map_err(|e| FetchError::Decode(e.to_string()))
}

// =============================================================================
// `velo::prefetch` — warm up a resource before the user navigates
// =============================================================================

/// Fire-and-forget pre-warm of a URL's payload (used by `<Link prefetch />`).
///
/// Issues a low-effort background `fetch(url)` so the resource is in the
/// browser's HTTP cache by the time the user navigates there, making route
/// data load instantly. The result is intentionally discarded and never blocks;
/// any failure is silently ignored.
///
/// This is the client-side hook point for route warm-up: once real per-`.wasm`
/// code-splitting lands, `<Link prefetch />` will use this same entry point to
/// pre-load the destination route's chunk instead of (or in addition to) the
/// raw payload.
pub fn prefetch(url: &str) {
    let url = url.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = crate::fetch(&url).await;
    });
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

/// How an individual `class_names!` argument contributes to the joined class
/// string. `None` (and empty strings) contribute nothing, so conditional
/// classes fall out naturally:
///
/// ```rust,ignore
/// class={ class_names!(
///     "btn",
///     is_active.then_some("is-active"),
///     size.map(|s| format!("btn-{s}")),
/// ) }
/// ```
pub trait ClassNames {
    /// The class(es) this value contributes, or `None` to contribute nothing.
    fn collect_class(&self) -> Option<String>;
}

impl ClassNames for String {
    fn collect_class(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            Some(self.clone())
        }
    }
}

impl ClassNames for &str {
    fn collect_class(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            Some(self.to_string())
        }
    }
}

impl<T: ClassNames> ClassNames for Option<T> {
    fn collect_class(&self) -> Option<String> {
        self.as_ref().and_then(|v| v.collect_class())
    }
}

/// Joins a conditional class list into a single space-separated `class` string.
/// `None` values and empty strings are skipped, so you can pass raw strings and
/// `Option`s together. Pairs with `class={ class_names!(..) }` in `view!`.
///
/// ```rust,ignore
/// view! {
///     <div class={ class_names!(
///         "card",
///         if selected.get() { "card--selected" } else { "card--dim" },
///         (n > 0).then_some("card--has-items"),
///     )}> { /* ... */ } </div>
/// }
/// ```
#[macro_export]
macro_rules! class_names {
    ($($arg:expr),* $(,)?) => {{
        vec![
            $( $crate::ClassNames::collect_class(&($arg)) ),*
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" ")
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

    /// Renders nothing. An alias for [`DomNode::fragment`] with zero
    /// children — appending it inserts no nodes at all, which is what you
    /// want for the "no-op" branch of a conditional render.
    ///
    /// Prefer this (or `view! { <></> }`, which expands to the same call)
    /// over `DomNode::text("")`: a fragment with no children leaves nothing
    /// behind in the DOM, whereas `text("")` still creates a real (empty)
    /// text node.
    ///
    /// ```ignore
    /// move || if details.get() {
    ///     view! { <div class="details">"Secret details"</div> }
    /// } else {
    ///     DomNode::empty()
    /// }
    /// ```
    pub fn empty() -> Self {
        Self::fragment()
    }

    /// Creates a persistent element that renders its children as if transparent
    /// (`display: contents`). Unlike a [`fragment`](DomNode::fragment), the
    /// element does **not** empty itself when appended to a parent, so reactive
    /// utilities can keep swapping children into it across effect runs.
    pub fn display_contents() -> Self {
        let el: Element = document().create_element("div").unwrap();
        el.set_attribute("style", "display: contents")
            .expect("Velo: Failed to set transparent container style");
        Self {
            raw_node: el.into(),
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

        // Establish the tracking wrapper loop. The effect is retained for the
        // node's lifetime (leaked): dropping an `EffectHandle` disposes the
        // effect, which would kill reactivity on the first update.
        std::mem::forget(create_effect(move || {
            let evaluated_string = f();
            current_node.set_node_value(Some(&evaluated_string));
        }));

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

        std::mem::forget(create_effect(move || {
            let value = f();
            el.set_attribute(&attr_name, &value)
                .expect("Velo: Failed to update element node attribute");
        }));
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

        std::mem::forget(create_effect(move || {
            let on = is_on();
            if on {
                let _ = el.class_list().add_1(&class_name);
            } else {
                let _ = el.class_list().remove_1(&class_name);
            }
        }));
    }

    /// Applies a static base class plus a set of reactively-toggled classes,
    /// coordinating every writer through one shared registry so they don't
    /// clobber each other.
    ///
    /// This is what the `view!` macro emits for an element that carries both a
    /// plain `class="..."` and one or more `class:name={ signal }` bindings. A
    /// naive approach writes each via a separate effect (`classList.add/remove`
    /// for toggles, `setAttribute("class", ...)` for the base) — and those two
    /// write the *same* `class` property, so whichever effect re-runs last
    /// wipes the other's classes. Here every contributor records into a single
    /// `Rc<RefCell<BTreeMap<name, bool>>>`, and any change rebuilds the full
    /// `className` from that map, so the base and all toggles always coexist.
    pub fn reactive_classes(
        &self,
        base: &str,
        toggles: Vec<(&'static str, Box<dyn FnMut() -> bool + 'static>)>,
    ) {
        use std::cell::RefCell;
        use std::collections::BTreeMap;
        use std::rc::Rc;
        use wasm_bindgen::JsCast;

        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: reactive_classes can only apply to element nodes");

        // Registry shared by every contributor. `order` fixes the class order
        // so the rebuilt className is deterministic (base first, then toggles
        // in declaration order).
        let state: Rc<RefCell<BTreeMap<String, bool>>> =
            Rc::new(RefCell::new(BTreeMap::new()));
        let mut order: Vec<String> = base.split_whitespace().map(str::to_string).collect();
        for (name, _) in &toggles {
            order.push((*name).to_string());
        }

        // Base classes are always on.
        {
            let mut st = state.borrow_mut();
            for c in &order {
                st.insert(c.clone(), true);
            }
        }

        let apply_state = Rc::clone(&state);
        let apply_order = order;
        let apply_el = el.clone();
        let apply = move || {
            let st = apply_state.borrow();
            let joined = apply_order
                .iter()
                .filter(|c| st.get(c.as_str()).copied().unwrap_or(false))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let _ = apply_el.set_class_name(&joined);
        };
        apply();

        for (name, mut is_on) in toggles {
            let name = name.to_string();
            let toggle_state = Rc::clone(&state);
            let toggle_apply = apply.clone();
            std::mem::forget(create_effect(move || {
                let on = is_on();
                toggle_state.borrow_mut().insert(name.clone(), on);
                toggle_apply();
            }));
        }
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

        std::mem::forget(create_effect(move || {
            let value = f();
            styles_c.borrow_mut().insert(property.clone(), value);

            let css: String = styles_c
                .borrow()
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = el.set_attribute("style", &css);
        }));
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
        // NOTE: FRender does NOT need + Clone. The closure is moved into the
        // effect once and called multiple times from within it — cloning would
        // force the closure to be Copy, which breaks non-Copy captures.
        // See https://github.com/velo-framework/velo/issues/... for context.
        FRender: Fn(&T) -> DomNode + 'static,
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

        // Retained (leaked) for the container's lifetime — dropping the handle
        // disposes the effect and freezes the list at its initial render.
        std::mem::forget(create_effect(move || {
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

            // Build nodes only for keys we don't already have. Re-rendering
            // every item on each mutation is wasteful: it re-runs (and leaks)
            // the per-item reactive effects even when nothing changed, which
            // degrades to quadratic time as the list grows.
            let mut by_key: HashMap<K, DomNode> = HashMap::new();
            let existing_keys: std::collections::HashSet<K> =
                nodes_c.borrow().keys().cloned().collect();
            for it in &items {
                let k = key(it);
                if !existing_keys.contains(&k) {
                    let node = render(it);
                    by_key.insert(k, node);
                }
            }

            // Reconcile against previous order: insert/move each item before the
            // node that precedes it in the previous layout (or append at end).
            let prev = order_c.borrow().clone();
            for (idx, k) in new_keys.iter().enumerate() {
                // Avoid holding a RefCell borrow across the match: the scrutinee
                // `borrow` temporary is dropped before the arm bodies run, so the
                // `borrow_mut()` in the `None` arm can't double-borrow.
                let existing = nodes_c.borrow().get(k).cloned();
                let node = match existing {
                    Some(existing) => existing,
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
        }));
    }

    /// Accepts a closure from the macro, evaluates it inside an effect loop,
    /// and resolves whether it's rendering a component or text dynamically!
    pub fn render_expression<F, R>(mut f: F) -> Self
    where
        F: FnMut() -> R + 'static,
        R: RenderDynamic + 'static,
    {
        // Use a persistent element container for dynamic expressions. A
        // DocumentFragment would be emptied into its parent on append, so later
        // effect runs would write into a detached fragment and never appear.
        // `display: contents` keeps the wrapper layout-transparent (like the old
        // fragment) while staying attached so swaps are visible.
        let container = DomNode::display_contents();
        let container_raw = container.raw_node.clone();

        let mut f_clone = move || f();

        // Retained (leaked) for the container's lifetime; see reactive_text.
        std::mem::forget(create_effect(move || {
            let val: R = f_clone();
            let resolved_node = val.render_dynamic();

            // Clear the existing content of the container.
            while let Some(child) = container_raw.first_child() {
                container_raw.remove_child(&child).unwrap();
            }

            container_raw
                .append_child(&resolved_node.raw_node)
                .expect("Velo: Failed to append dynamic expression variant");
        }));

        container
    }
}

/// Reactive two-branch control flow backing `<Show>` / `<Suspense>`.
///
/// `content` and `fallback` are pre-built `DomNode` subtrees (each already
/// internally reactive). This function shows whichever branch is active and
/// swaps between them whenever the reactive `when` predicate flips — so an
/// async resource's `loading` signal automatically swaps fallback <-> content
/// without rebuilding children. Original JS nodes are moved in/out, which keeps
/// any nested reactive expressions wired to the *same* live DOM nodes.
pub fn reactive_switch<W>(mut when: W, content: DomNode, fallback: DomNode) -> DomNode
where
    W: FnMut() -> bool + 'static,
{
    // A persistent `display: contents` container: stays attached so the branch
    // swap is visible on re-run (a DocumentFragment would empty on append).
    let container = DomNode::display_contents();
    let container_raw = container.raw_node.clone();
    let content_raw = content.raw_node.clone();
    let fallback_raw = fallback.raw_node.clone();

    // Retained (leaked) for the fragment's lifetime; see reactive_text.
    std::mem::forget(create_effect(move || {
        while let Some(child) = container_raw.first_child() {
            let _ = container_raw.remove_child(&child);
        }
        let active = if when() {
            content_raw.clone()
        } else {
            fallback_raw.clone()
        };
        let _ = container_raw.append_child(&active);
    }));

    container
}

/// Async lazy-loader returning a `DomNode`: shows `fallback` (a loading
/// placeholder) immediately, then **swaps in** the real node once the async
/// `loader` future resolves. This is the client-side analogue of Next.js
/// `next/dynamic` / React `lazy` — the primitive and hook point for the
/// route-based code-splitting roadmap (5.P8).
///
/// ```rust,ignore
/// // A heavy page section that loads asynchronously with a visible spinner:
/// <Suspense loading={ r.loading() } fallback={ view! { <p>"Loading…"</p> } }>
///     { use_dynamic(|| async {
///         velo::sleep(400).await;          // pretend to fetch a chunk
///         view! { <Chart data={ data } /> }
///     }) }
/// </Suspense>
/// ```
///
/// The returned node is attached to the live tree as soon as its caller
/// renders, and the caller's own subtree stays reactive across the swap (the
/// loader's resolved node is moved into place, not rebuilt).
pub fn use_dynamic<F, Fut>(loader: F, fallback: DomNode) -> DomNode
where
    F: FnOnce() -> Fut + 'static,
    Fut: std::future::Future<Output = DomNode> + 'static,
{
    let container = DomNode::display_contents();
    let container_raw = container.raw_node.clone();
    if let Err(_) = container_raw.append_child(&fallback.raw_node) {}

    wasm_bindgen_futures::spawn_local(async move {
        let node = loader().await;
        while let Some(child) = container_raw.first_child() {
            let _ = container_raw.remove_child(&child);
        }
        let _ = container_raw.append_child(&node.raw_node);
    });

    container
}

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
        // Don't automatically unmount on drop - the node stays mounted in the DOM.
        // User must explicitly call .unmount() to remove it.
        // This allows `mount()` to be called without storing the handle.
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
    install_dev_overlay();
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
    install_dev_overlay();
    target
        .append_child(&root.raw_node)
        .expect("Velo: Failed to mount root node into target");
    RootHandle { root }
}

// ---------------------------------------------------------------------------
// Built-in dev error overlay  (§5.P10)
// ---------------------------------------------------------------------------
//
// The overlay is injected at runtime on the first `mount()`/`mount_at()`. It
// subscribes to Trunk's dev WebSocket (`/.well-known/trunk/ws`) and surfaces
// compile failures as a styled panel instead of a dead tab. It is a no-op
// whenever the app is NOT served by `trunk serve` (the WebSocket never
// connects), so it is safe to install unconditionally and requires zero
// per-project setup — no script tag, no asset copy, no config.

use std::sync::atomic::{AtomicBool, Ordering};
static DEV_OVERLAY_INSTALLED: AtomicBool = AtomicBool::new(false);

/// The compiled-in JS for Velo's dev error overlay. A human-readable,
/// editable reference copy lives at `docs/templates/velo-error-overlay.js`;
/// keep the two in sync if behavior changes.
const DEV_OVERLAY_JS: &str = r##"(() => {
  if (window.__veloDevOverlay) return;
  window.__veloDevOverlay = true;
  const guardStyle = document.createElement("style");
  guardStyle.textContent = 'div[style*="rgba(222, 222, 222, 0.5)"]{display:none !important;}';
  document.head.appendChild(guardStyle);
  let panel = null;
  const wsUrl = () => {
    const proto = location.protocol === "https:" ? "wss://" : "ws://";
    return proto + location.host + "/.well-known/trunk/ws";
  };
  const buildPanel = () => {
    const root = document.createElement("div");
    root.id = "velo-dev-overlay";
    root.setAttribute("style", "position:fixed;inset:0;z-index:2147483000;display:flex;align-items:center;justify-content:center;padding:2rem;background:rgba(2,6,23,.72);backdrop-filter:blur(6px);font-family:system-ui,-apple-system,sans-serif;color:#e2e8f0;");
    const card = document.createElement("div");
    card.setAttribute("style", "max-width:min(880px,100%);width:100%;max-height:85vh;overflow:auto;border:1px solid #7f1d1d;border-radius:14px;background:#111827;box-shadow:0 24px 60px rgba(0,0,0,.5);");
    const head = document.createElement("div");
    head.setAttribute("style", "display:flex;align-items:center;gap:.75rem;padding:1rem 1.25rem;border-bottom:1px solid #374151;background:#1f2937;border-radius:14px 14px 0 0;");
    const icon = document.createElement("span");
    icon.innerHTML = '<svg width="22" height="22" viewBox="0 0 16 16" fill="none"><path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566z" fill="#f87171"/><path d="M8 5.5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 6.495A.905.905 0 0 1 8 5.5zm.002 5.5a1 1 0 1 1 0 2 1 1 0 0 1 0-2z" fill="#111827"/></svg>';
    const title = document.createElement("span");
    title.textContent = "Build failed";
    title.setAttribute("style", "font-size:1.05rem;font-weight:700;color:#fca5a5");
    const close = document.createElement("button");
    close.type = "button"; close.setAttribute("aria-label", "Dismiss"); close.textContent = "\u00d7";
    close.setAttribute("style", "margin-left:auto;background:transparent;border:none;color:#94a3b8;font-size:1.5rem;cursor:pointer;line-height:1;padding:.25rem .6rem;border-radius:8px;");
    close.addEventListener("mouseenter", () => close.style.color = "#f1f5f9");
    close.addEventListener("mouseleave", () => close.style.color = "#94a3b8");
    const msg = document.createElement("pre");
    msg.id = "velo-dev-overlay-msg";
    msg.setAttribute("style", "margin:1rem 1.25rem;white-space:pre-wrap;word-break:break-word;font:0.83rem/1.55 ui-monospace,SFMono-Regular,Menlo,monospace;color:#fbbf24;");
    const foot = document.createElement("div");
    foot.setAttribute("style", "padding:.6rem 1.25rem;border-top:1px solid #1f2937;color:#64748b;font-size:.78rem;");
    foot.textContent = "Velo dev overlay \u00b7 full diagnostic (file.rs:line:col) is in the trunk server terminal \u00b7 save the fix and this page reloads automatically.";
    head.append(icon, title, close);
    card.append(head, msg, foot);
    root.appendChild(card);
    document.body.appendChild(root);
    const dismiss = () => { root.remove(); panel = null; };
    close.addEventListener("click", dismiss);
    root.addEventListener("click", (ev) => { if (ev.target === root) dismiss(); });
    return root;
  };
  let reloading = false;
  const tryReload = () => { if (reloading) return; reloading = true; window.location.reload(); setTimeout(() => reloading = false, 1000); };
  let ws;
  try { ws = new WebSocket(wsUrl()); } catch (_e) { return; }
  ws.onopen = () => { ws.onclose = () => tryReload(); };
  ws.onerror = () => ws.close();
  ws.onmessage = (ev) => {
    let m; try { m = JSON.parse(ev.data); } catch (_e) { return; }
    if (m.type === "reload") tryReload();
    else if (m.type === "buildFailure") {
      console.error("Velo dev overlay: build failed\n" + m.data.reason);
      if (!panel) panel = buildPanel();
      document.getElementById("velo-dev-overlay-msg").textContent = m.data.reason || "Unknown build failure";
    }
  };
})();"##;

/// Inject Velo's built-in dev error overlay (idempotent, no-op outside
/// `trunk serve`). Called automatically by [`mount`]/[`mount_at`].
pub fn install_dev_overlay() {
    if DEV_OVERLAY_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = js_sys::eval(DEV_OVERLAY_JS);
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

/// Static, non-reactive conditional rendering (direct API only).
///
/// The `view!` macro compiles `<Show>` / `<Suspense>` to [`reactive_switch`]
/// instead, which swaps branches when the condition changes. This function is
/// a plain one-shot helper for code that doesn't need reactivity.
#[allow(non_snake_case)]
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

// =============================================================================
// Persistent layout shells  (§4 / 5.P4)
// =============================================================================

/// A layout layer: wraps a matched child subtree
/// (`fn layout(child: DomNode) -> DomNode`). `app!` registers the chain per
/// route path so the [`Router`] keeps the shell mounted across sibling
/// navigation and swaps only the leaf outlet.
pub type LayoutFn = fn(DomNode) -> DomNode;

struct LayoutRegistrationRow {
    path: String,
    layouts: Vec<LayoutFn>,
}

thread_local! {
    static LAYOUT_REGISTRY: RefCell<Vec<LayoutRegistrationRow>> = RefCell::new(Vec::new());
}

/// Register the `src/app/` layout chain for each route path (emitted by
/// `velo::app!`'s generated `routes()`). Idempotent: a path is only stored once.
pub fn register_app_layouts(entries: &[(&'static str, &[LayoutFn])]) {
    LAYOUT_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        for (path, layouts) in entries {
            if !reg.iter().any(|r| r.path == *path) {
                reg.push(LayoutRegistrationRow {
                    path: path.to_string(),
                    layouts: layouts.to_vec(),
                });
            }
        }
    });
}

/// Layout chain (nearest segment layout -> ... -> root layout) registered for
/// a route path; empty for routes without file-based `app!` layouts.
pub fn app_layouts(path: &str) -> Vec<LayoutFn> {
    LAYOUT_REGISTRY.with(|reg| {
        reg.borrow()
            .iter()
            .find(|r| r.path == path)
            .map(|r| r.layouts.clone())
            .unwrap_or_default()
    })
}

/// True between a navigation starting and the newly mounted route's first
/// microtask — drives the automatic `loading.rs` placeholder emitted by `app!`.
pub fn route_loading() -> bool {
    ROUTE_LOADING.with(|s| s.get())
}

thread_local! {
    static ROUTE_LOADING: Signal<bool> = Signal::new(false);
}

pub(crate) fn mark_route_loading() {
    ROUTE_LOADING.with(|s| s.set(true));
}

pub(crate) fn clear_route_loading() {
    ROUTE_LOADING.with(|s| {
        let slot = s.clone();
        wasm_bindgen_futures::spawn_local(async move {
            slot.set(false);
        });
    });
}

// =============================================================================
// Error boundaries  (§ 5.P5)
// =============================================================================

/// A default built-in fallback pane used when a route defines no `error.rs`.
pub fn default_error_fallback() -> DomNode {
    let div = DomNode::element("div");
    div.reactive_attribute("class", move || "velo-error-boundary".to_string());
    let p = DomNode::element("p");
    p.append(&DomNode::text("Something went wrong rendering this subtree."));
    div.append(&p);
    div
}

thread_local! {
    /// Active `error_boundary` status signals, nearest first. `boundary_fault`
    /// writes to the current top of the stack; the boundary that owns it swaps
    /// in its fallback. Nestable (a fault is consumed by the closest boundary).
    static BOUNDARY_STACK: RefCell<Vec<RwSignal<Option<String>>>> = RefCell::new(Vec::new());
}

/// Declare the current error-boundary'd subtree as failed with a message.
/// Purely "app-level": it requires no unwinding, so it works on wasm where
/// `panic = "abort"` makes catching real panics impossible. The nearest
/// enclosing [`error_boundary`] renders its fallback instead of this subtree
/// and the rest of the app keeps running. Returns a throwaway node so it can
/// be used as a return expression in a `page()`/component fn.
pub fn boundary_fault(message: impl Into<String>) -> DomNode {
    let msg = message.into();
    BOUNDARY_STACK.with(|s| {
        if let Some(top) = s.borrow().last() {
            top.set(Some(msg));
        }
    });
    DomNode::fragment()
}

/// Renders `build()` guarded by an error boundary: if the subtree calls
/// [`boundary_fault`] (the wasm-compatible "Result from a resource" path), or
/// unwinds a panic on targets that support it (`catch_unwind` on native), the
/// `fallback` is shown instead and the rest of the app keeps living. This is
/// exactly what `app!` emits for every page, gated by the nearest `error.rs`.
///
/// **Note:** `wasm32-unknown-unknown` compiles with `panic = "abort"` (no
/// unwinding), so genuine `panic!`s there cannot be recovered — use
/// [`boundary_fault`] for error boundaries that must survive on wasm.
pub fn error_boundary(fallback: DomNode, build: Box<dyn FnOnce() -> DomNode + 'static>) -> DomNode {
    let status: RwSignal<Option<String>> = RwSignal::new(None);

    BOUNDARY_STACK.with(|s| s.borrow_mut().push(status.clone()));

    // On native this also catches real unwinding panics inside the subtree.
    let built =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(Box::new(move || build())));

    BOUNDARY_STACK.with(|s| {
        s.borrow_mut().pop();
    });

    match (built, status.get()) {
        (Ok(node), None) => node,
        _ => fallback,
    }
}

// =============================================================================
// Compile-time route collection via `inventory`
// =============================================================================

/// Re-exported so the `#[route = "..."]` macro (in `velo_macro`) can emit
/// `velo::inventory::submit! { .. }` without every consuming crate having to
/// add `inventory` as its own direct dependency.
pub use inventory;

/// One statically-registered route. Every `#[route = "/path"]`-annotated
/// function submits one of these at compile time; collect them all with
/// [`collected_routes`] instead of hand-building a `Vec<Route>`.
pub struct RouteRegistration {
    pub path: &'static str,
    pub component: fn() -> DomNode,
}

inventory::collect!(RouteRegistration);

/// Gather every route registered via `#[route = "..."]` into a `Vec<Route>`,
/// ready to hand to `<Router routes={collected_routes()} />` or
/// `FRouter::new(...)`.
///
/// ```ignore
/// #[route("/users/:id")]
/// pub fn user_profile_page() {
///     view! { <div>"User " { FRouter::param("id").unwrap_or_default() }</div> }
/// }
///
/// // Instead of hand-writing the Vec<Route> list:
/// mount(view! { <Router routes={collected_routes()} /> });
/// ```
pub fn collected_routes() -> Vec<Route> {
    inventory::iter::<RouteRegistration>()
        .map(|r| Route {
            path: r.path,
            component: r.component,
        })
        .collect()
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
        std::mem::forget(create_effect(move || {
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
        }));

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
    view_wrapper.reactive_attribute("class", move || "velo-router-viewport".to_string());

    // Stable leaf outlet (M4 / 5.P4): the `app!` layout shell from
    // `register_app_layouts` wraps THIS node; navigating between sibling routes
    // swaps only the leaf inside it, so the layout subtree stays mounted and
    // its local state (signals, scroll) is preserved.
    let outlet = DomNode::element("div");
    outlet.reactive_attribute("data-velo-route", move || "leaf".to_string());
    let outlet_raw = outlet.raw_node.clone();

    let shell_holder: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));
    let leaf_holder: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));
    let active_chain: Rc<RefCell<Vec<LayoutFn>>> = Rc::new(RefCell::new(Vec::new()));

    let wrapper_raw = view_wrapper.raw_node.clone();
    let shell_c = Rc::clone(&shell_holder);
    let leaf_c = Rc::clone(&leaf_holder);
    let chain_c = Rc::clone(&active_chain);
    let outlet_c = outlet.clone();

    std::mem::forget(create_effect(move || {
        let current_path = CURRENT_PATH.with(|p| p.get());

        // 1. Match the route, staging parsed params BEFORE the leaf renders so
        //    `FRouter::param` / `use_param` read fresh values.
        let mut params_payload = HashMap::new();
        let matched = routes.iter().find(|r| {
            if let Some(map) = match_route_patterns(r.path, &current_path) {
                params_payload = map;
                true
            } else {
                false
            }
        });

        ACTIVE_PARAMS.with(|p| {
            *p.borrow_mut() = params_payload;
        });

        // 2. Layout shell: rebuild ONLY when the chain identity changes.
        //    Same chain (e.g. `/blog/:slug` -> `/blog/:slug`) -> keep the
        //    existing mounted shell; a layout-boundary change tears it down.
        //    On the first run (shell_c is None), always append the shell so the
        //    leaf outlet is actually inside the wrapper and visible in the DOM.
        let chain = matched.map(|r| app_layouts(r.path)).unwrap_or_default();
        if chain != *chain_c.borrow() || shell_c.borrow().is_none() {
            if let Some(old) = shell_c.borrow().as_ref() {
                let _ = wrapper_raw.remove_child(&old.raw_node);
            }
            // The same outlet is reused across rebuilds (it's just a Node);
            // a rebuilt shell around it resets layout-local state, matching
            // Next.js segment-layout remount semantics.
            let mut shell = outlet_c.clone();
            for l in &chain {
                shell = l(shell);
            }
            let _ = wrapper_raw.append_child(&shell.raw_node);
            *shell_c.borrow_mut() = Some(shell);
            *chain_c.borrow_mut() = chain;
        }

        // 3. Signal a loading window so `app!`-emitted `loading.rs` placeholders
        //    can flash until the freshly rendered route is attached.
        mark_route_loading();

        // 4. Swap the leaf inside the stable outlet.
        if let Some(old) = leaf_c.borrow().as_ref() {
            let _ = outlet_raw.remove_child(&old.raw_node);
        }
        let new_leaf = match matched {
            Some(route) => (route.component)(),
            None => {
                let mut fallback = DomNode::element("h1");
                fallback.append(&DomNode::text("404 - Page Not Found"));
                fallback
            }
        };
        let _ = outlet_raw.append_child(&new_leaf.raw_node);
        *leaf_c.borrow_mut() = Some(new_leaf);

        clear_route_loading();
    }));

    view_wrapper
}

/// Boundary-safe active-route match for `<Link active_class>`.
///
/// `to == "/"` activates only on the exact root path; any other `to` activates
/// when `current == to` or `current` is a deeper descendant of `to`
/// (i.e. `current.starts_with(to + "/")`), so `/blog` matches `/blog` and
/// `/blog/:slug` but never `/blogxyz`.
pub fn is_path_active(current: &str, to: &str) -> bool {
    if to == "/" {
        return current == "/";
    }
    if current == to {
        return true;
    }
    let prefix = if to.ends_with('/') {
        to.to_string()
    } else {
        format!("{to}/")
    };
    current.starts_with(&prefix)
}

// ---------------------------------------------------------------------------
// <Head> — reactive per-route document title & meta (§5.P6)
// ---------------------------------------------------------------------------
//
// Drop a `<Head>` inside any route (or layout) to set `document.title` (and
// optional `<meta name>` tags) whenever that node renders. Because the Router
// re-renders the matched leaf on navigation, a `<Head>` placed in each route
// updates the browser title/meta on every navigation — the client-side SPA
// analogue of Next.js `layout.tsx`/`page.tsx` metadata.

/// Props for [`Head`]: `<Head title="My App" meta={ vec![("description", "...")] } />`.
#[allow(non_snake_case)]
pub struct HeadProps {
    /// The document title to set. Omit to leave `document.title` untouched.
    pub title: Option<String>,
    /// Optional `(name, content)` pairs rendered as `<meta name=.. content=..>`.
    pub meta: Option<Vec<(String, String)>>,
}

thread_local! {
    // Names of `data-velo-meta` elements owned by the most recent <Head>, so a
    // fresh navigation can clear the previous route's tags instead of stacking.
    static HEAD_META_TAGS: RefCell<Vec<web_sys::Element>> = RefCell::new(Vec::new());
}

#[allow(non_snake_case)]
pub fn Head(props: HeadProps) -> DomNode {
    let HeadProps { title, meta } = props;

    if let Some(title) = title {
        let doc = document();
        if let Some(t) = doc.query_selector("title").ok().flatten() {
            t.set_text_content(Some(&title));
        } else {
            let t = doc.create_element("title").expect("Velo: create title element");
            t.set_text_content(Some(&title));
            if let Some(head) = doc.query_selector("head").ok().flatten() {
                head.append_child(&t).ok();
            }
        }
    }

    if let Some(meta_list) = meta {
        let doc = document();
        let head = doc
            .query_selector("head")
            .ok()
            .flatten()
            .expect("Velo: document head");
        // Remove tags left by a previous <Head> navigation so metas never stack.
        HEAD_META_TAGS.with(|tags| {
            let old = std::mem::take(&mut *tags.borrow_mut());
            for el in old {
                let _ = el.remove();
            }
        });
        let holder: Vec<web_sys::Element> = meta_list
            .into_iter()
            .map(|(name, content)| {
                let m = doc
                    .create_element("meta")
                    .expect("Velo: create meta element");
                m.set_attribute("name", &name).ok();
                m.set_attribute("content", &content).ok();
                m.set_attribute("data-velo-meta", "").ok();
                head.append_child(&m).ok();
                m
            })
            .collect();
        HEAD_META_TAGS.with(|tags| *tags.borrow_mut() = holder);
    }

    // <Head> never renders anything into <body>.
    DomNode::empty()
}

/// Props for [`Link`]: `<Link to="..." label="..." />` or `<Link to="...">Children</Link>`.
/// Supports active state styling via the `active_class` prop and on-hover route
/// pre-warm-up via the `prefetch` prop.
#[allow(non_snake_case)]
pub struct LinkProps {
    /// Destination path. Accepts a typed `paths::*` builder from `velo::app!`
    /// (or the new `routes!`-free router); string literals coerce via `.into()`.
    pub to: String,
    /// Optional static label text (used when no children are provided).
    pub label: Option<&'static str>,
    /// Optional children nodes (takes precedence over `label`).
    pub children: Option<Vec<DomNode>>,
    /// Optional CSS class to apply when this link's route is active.
    pub active_class: Option<&'static str>,
    /// When `true`, hovering over (or focusing) the link pre-warms the
    /// destination's payload in the background so navigation feels instant.
    /// Defaults to `false`.
    pub prefetch: bool,
}

/// Ergonomic Link component that allows clean macro nesting: <Link to="...">Children</Link>
/// Supports active state styling via `active_class` prop.
#[allow(non_snake_case)]
pub fn Link(props: LinkProps) -> DomNode {
    let LinkProps {
        to,
        label,
        children,
        active_class,
        prefetch: prefetch_enabled,
    } = props;
    let anchor = DomNode::element("a");
    let href = to.clone();
    anchor.reactive_attribute("href", move || href.clone());

    // Render children if provided, otherwise use label text
    if let Some(children) = children {
        for child in children {
            anchor.append(&child);
        }
    } else if let Some(label) = label {
        let text_content = DomNode::text(label);
        anchor.append(&text_content);
    }

    // Active state: if active_class provided, reactively add/remove it based on
    // the current route. Matching is boundary-safe: `/blog` only activates for
    // `/blog` or `/blog/...`, never `/blogxyz`. Root (`/`) activates only when
    // the current path is exactly `/`.
    if let Some(active_class) = active_class {
        let to = to.to_string();
        anchor.reactive_attribute("class", move || {
            let current = FRouter::use_route();
            if is_path_active(&current, &to) {
                active_class.to_string()
            } else {
                String::new()
            }
        });
    }

    // Prefetch: on hover/focus, warm up the destination's payload in the
    // background the first time (so it's cached before a click navigates).
    if prefetch_enabled {
        let to = to.clone();
        let to_focus = to.clone();
        anchor.on("mouseenter", move |_e| prefetch(&to));
        anchor.on("focus", move |_e| prefetch(&to_focus));
    }

    anchor.on("click", move |event| {
        event.prevent_default();
        navigate_to(&to);
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
    // Re-export wasm-bindgen-futures so the view! macro's generated
    // `wasm_bindgen_futures::spawn_local(...)` for `async () => { .. }` handlers
    // resolves under `use velo::prelude::*` without per-example boilerplate.
    pub use wasm_bindgen_futures;

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
        sleep,
        use_context,
        use_dynamic,
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

    // Re-export the class_names! join helper + its trait
    #[doc(hidden)]
    pub use crate::ClassNames;
    pub use crate::class_names;

    // Re-export primary DOM manipulation types and helpers
    pub use crate::{
        document, mount, mount_at, mount_to_id, DomNode, RenderDynamic, RootHandle,
    };

    // Re-export router structures
    pub use crate::{
        app_layouts, boundary_fault, collected_routes, default_error_fallback, error_boundary,
        FRouter, Head, HeadProps, is_path_active, LayoutFn, Link, LinkProps, navigate_to,
        register_app_layouts, Route, RouteRegistration, Router, RouterProps,
    };

    // Re-export the JS `fetch` sugar (fetch / response body / typed JSON)
    pub use crate::{fetch, prefetch, FetchError, VeloResponse};
    #[cfg(feature = "json")]
    pub use crate::fetch_json;

    // Re-export the view! + #[component] + routes! + #[route] + app!/#[page]
    // procedural macros
    pub use crate::{app, component, error, layout, loading, not_found, page, route, routes, route_path, view};

    // Re-export the shorthand convenience macros. `signal!` shares its name with
    // the `signal` value already re-exported above, so the macro rides along on
    // that single import (macro + value namespaces are both covered by one use).
    pub use crate::{context, effect, provide};

    // Re-export the built-in control-flow component `Show`.
    pub use crate::Show;
}
