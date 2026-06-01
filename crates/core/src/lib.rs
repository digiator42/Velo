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

/// The core primitive structure containing data that dynamically changes over time
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    // We store the weak/RC references to the effects safely
    subscribers: Rc<RefCell<Vec<Rc<RefCell<Effect>>>>>,
}

// Manual implementation of Clone to clone pointers, not data
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(initial_value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(initial_value)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn get(&self) -> T {
        ACTIVE_EFFECT_ID.with(|active_id| {
            if let Some(current_id) = *active_id.borrow() {
                // Find the actual effect structure inside the global registry to subscribe it
                EFFECT_REGISTRY.with(|registry| {
                    if let Some(effect_rc) = registry
                        .borrow()
                        .iter()
                        .find(|e| e.borrow().id == current_id)
                    {
                        let mut subs = self.subscribers.borrow_mut();
                        if !subs.iter().any(|s| s.borrow().id == current_id) {
                            subs.push(Rc::clone(effect_rc));
                        }
                    }
                });
            }
        });
        self.value.borrow().clone()
    }

    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value;
        self.notify();
    }

    fn notify(&self) {
        let subs = self.subscribers.borrow().clone();
        let mut executed: HashSet<EffectId> = HashSet::new();

        for effect_rc in subs {
            let effect_id = effect_rc.borrow().id;
            if !executed.contains(&effect_id) {
                executed.insert(effect_id);

                // Set the active tracking context ID safely
                let previous_id = ACTIVE_EFFECT_ID.with(|active| active.replace(Some(effect_id)));

                // Extract the function loop out of the RefCell to execute it safely without leaving it locked
                let mut func = std::mem::replace(&mut effect_rc.borrow_mut().func, Box::new(|| {}));
                (func)();
                // Put the function loop cleanly back into place
                effect_rc.borrow_mut().func = func;

                ACTIVE_EFFECT_ID.with(|active| active.replace(previous_id));
            }
        }
    }

    pub fn mount_effect(&self, effect: Rc<RefCell<Effect>>) {
        let mut subs = self.subscribers.borrow_mut();
        if !subs.iter().any(|s| s.borrow().id == effect.borrow().id) {
            subs.push(effect);
        }
    }
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
}
