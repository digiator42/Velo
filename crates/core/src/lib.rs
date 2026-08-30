use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(usize);

thread_local! {
    // Tracks just the ID of what is currently running to prevent recursive borrow conflicts
    static ACTIVE_EFFECT_ID: RefCell<Option<EffectId>> = RefCell::new(None);

    // A registry linking running IDs back to their executable updates
    static EFFECT_REGISTRY: RefCell<Vec<Rc<RefCell<Effect>>>> = RefCell::new(Vec::new());

    static EFFECT_COUNTER: RefCell<usize> = RefCell::new(0);
}

pub struct Effect {
    id: EffectId,
    func: Box<dyn FnMut()>,
}

/// The shared reactive cell backing every signal handle.
/// Cloning a handle clones the `Rc` pointers, never the data.
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
                        .find(|e| e.borrow().id == current_id)
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

    fn notify(&self) {
        let subs = self.subscribers.borrow().clone();
        let mut executed: HashSet<EffectId> = HashSet::new();

        for effect_rc in subs {
            let effect_id = effect_rc.borrow().id;
            if !executed.contains(&effect_id) {
                executed.insert(effect_id);

                let previous_id = ACTIVE_EFFECT_ID.with(|active| active.replace(Some(effect_id)));

                let mut func = std::mem::replace(&mut effect_rc.borrow_mut().func, Box::new(|| {}));
                (func)();
                effect_rc.borrow_mut().func = func;

                ACTIVE_EFFECT_ID.with(|active| active.replace(previous_id));
            }
        }
    }

    fn mount_effect(&self, effect: &Rc<RefCell<Effect>>) {
        let mut subs = self.subscribers.borrow_mut();
        if !subs.iter().any(|s| s.borrow().id == effect.borrow().id) {
            subs.push(Rc::clone(effect));
        }
    }
}

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

/// Create a reactive signal, returning a `(read, write)` handle pair.
///
/// ```ignore
/// let (count, set_count) = create_signal(0);
/// set_count.set(count.get() + 1);
/// ```
pub fn create_signal<T: Clone + 'static>(initial_value: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let inner = SignalInner::new(initial_value);
    (
        ReadSignal {
            inner: inner.clone(),
        },
        WriteSignal { inner },
    )
}

/// A derived, cached read-only signal. The closure runs inside an effect, so it
/// automatically re-computes whenever any signal it reads changes.
pub fn create_memo<F, T>(mut f: F) -> ReadSignal<T>
where
    F: FnMut() -> T + 'static,
    T: Clone + 'static,
{
    let init = f();
    let (read, write) = create_signal(init);
    create_effect({
        let write = write.clone();
        move || {
            let next = f();
            write.set(next);
        }
    });
    read
}

pub fn create_effect<F>(func: F)
where
    F: FnMut() + 'static,
{
    let next_id = EFFECT_COUNTER.with(|counter| {
        let mut c = counter.borrow_mut();
        *c += 1;
        *c
    });

    let id = EffectId(next_id);
    let effect = Rc::new(RefCell::new(Effect {
        id,
        func: Box::new(func),
    }));

    // Register globally
    EFFECT_REGISTRY.with(|registry| registry.borrow_mut().push(Rc::clone(&effect)));

    // Initial seed evaluation execution run
    let previous_id = ACTIVE_EFFECT_ID.with(|active| active.replace(Some(id)));

    let mut func = std::mem::replace(&mut effect.borrow_mut().func, Box::new(|| {}));
    (func)();
    effect.borrow_mut().func = func;

    ACTIVE_EFFECT_ID.with(|active| active.replace(previous_id));
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

    /// Subscribe a callback that receives the (cloned) current items. Unlike
    /// `create_effect`, this hands you the whole list so the DOM layer can diff.
    pub fn subscribe<F: FnMut(Vec<T>) + 'static>(&self, mut f: F) {
        let inner = self.inner.clone();
        create_effect(move || {
            let items = inner.get();
            f(items);
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fine_grained_reactivity() {
        let count = Signal::new(10);
        let trigger_count = Rc::new(RefCell::new(0));

        let tc = Rc::clone(&trigger_count);
        let c = count.clone();

        create_effect(move || {
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

        create_effect(move || {
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

        create_effect(move || {
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
}
