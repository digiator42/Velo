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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fine_grained_reactivity() {
        let count = Signal::new(10);
        let trigger_count = Rc::new(RefCell::new(0));

        let tc = Rc::clone(&trigger_count);
        let c = count.clone();

        let _handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _current = c.get();
        });

        assert_eq!(*trigger_count.borrow(), 1);

        count.set(20);
        assert_eq!(*trigger_count.borrow(), 2);

        count.set(30);
        assert_eq!(*trigger_count.borrow(), 3);
    }

    #[test]
    fn test_split_signal() {
        let (count, set_count) = create_signal(0);
        let trigger = Rc::new(RefCell::new(0));
        let tc = Rc::clone(&trigger);
        let c = count.clone();

        let _handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _ = c.get();
        });

        assert_eq!(*trigger.borrow(), 1);
        set_count.set(5);
        assert_eq!(*trigger.borrow(), 2);
        assert_eq!(count.get(), 5);
    }

    #[test]
    fn test_memo_recomputes_on_dependency_change() {
        let (base, set_base) = create_signal(2);
        let doubled = create_memo({
            let base = base.clone();
            move || base.get() * 2
        });

        assert_eq!(doubled.get(), 4);
        set_base.set(5);
        assert_eq!(doubled.get(), 10);
    }

    #[test]
    fn test_signal_vec_notifies_on_push() {
        let list = SignalVec::new(vec![1, 2, 3]);
        let trigger = Rc::new(RefCell::new(0));
        let tc = Rc::clone(&trigger);
        let l = list.clone();

        let _handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _ = l.get();
        });

        assert_eq!(*trigger.borrow(), 1);
        list.push(4);
        assert_eq!(*trigger.borrow(), 2);
        assert_eq!(list.len(), 4);
    }

    #[test]
    fn test_context_provides_value() {
        provide_context(42i32);
        assert_eq!(use_context::<i32>(), Some(42));

        let scoped = with_context("hi".to_string(), || use_context::<String>());
        assert_eq!(scoped, Some("hi".to_string()));

        // Outer context still intact after scoping.
        assert_eq!(use_context::<i32>(), Some(42));
        assert_eq!(use_context::<String>(), None);
    }

    #[test]
    fn test_dispose_effect_stops_notification() {
        let count = Signal::new(0);
        let trigger = Rc::new(RefCell::new(0));
        let tc = Rc::clone(&trigger);
        let c = count.clone();

        let handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _ = c.get();
        });

        assert_eq!(*trigger.borrow(), 1);
        count.set(1);
        assert_eq!(*trigger.borrow(), 2);

        // Dispose the effect; further changes should NOT trigger it.
        drop(handle);
        count.set(2);
        assert_eq!(*trigger.borrow(), 2); // unchanged
    }

    #[test]
    fn test_effect_cleanup_runs_on_dispose() {
        let cleanup_flag = Rc::new(RefCell::new(false));

        let handle = create_effect_with_cleanup(
            || {},
            {
                let cf = Rc::clone(&cleanup_flag);
                move || {
                    *cf.borrow_mut() = true;
                }
            },
        );

        let before = *cleanup_flag.borrow();
        assert!(!before);
        drop(handle);
        let after = *cleanup_flag.borrow();
        assert!(after);
    }

    #[test]
    fn test_batch_groups_notifications() {
        let (a, set_a) = create_signal(0);
        let (b, set_b) = create_signal(0);
        let trigger = Rc::new(RefCell::new(0));
        let tc = Rc::clone(&trigger);
        let a = a.clone();
        let b = b.clone();

        let _handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _ = a.get();
            let _ = b.get();
        });

        // Initial run.
        assert_eq!(*trigger.borrow(), 1);

        // Without batch: each set triggers separately.
        set_a.set(1);
        assert_eq!(*trigger.borrow(), 2);
        set_b.set(1);
        assert_eq!(*trigger.borrow(), 3);

        // Reset.
        *trigger.borrow_mut() = 0;

        // With batch: both sets before flush = one trigger.
        batch(|| {
            set_a.set(2);
            set_b.set(2);
        });
        assert_eq!(*trigger.borrow(), 1);
    }

    #[test]
    fn test_nested_batch_is_no_op() {
        let (x, set_x) = create_signal(0);
        let trigger = Rc::new(RefCell::new(0));
        let tc = Rc::clone(&trigger);
        let x = x.clone();

        let _handle = create_effect(move || {
            *tc.borrow_mut() += 1;
            let _ = x.get();
        });

        batch(|| {
            batch(|| {
                set_x.set(1);
            });
        });
        // Should flush exactly once despite two batch() calls.
        assert_eq!(*trigger.borrow(), 1);
    }
}
