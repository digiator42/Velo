use velo::prelude::*;

/// A single to-do item. Held inside a `SignalVec<Task>` so list mutations are
/// reactive and get reconciled by stable `id`.
#[derive(Clone)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub done: bool,
}

/// Application-wide state shared via context (no prop drilling).
#[derive(Clone)]
pub struct AppState {
    pub tasks: SignalVec<Task>,
}

/// Renders one task row. Reads the shared `AppState` from context so the list's
/// per-item render closure captures nothing non-`Copy` (required by the keyed
/// list reconciler) and stays `Copy` itself.
#[allow(non_snake_case)]
#[component]
pub fn TaskRow(task: Task) {
    let state = use_context::<AppState>().expect("AppState must be provided at root");
    let id = task.id;
    let tasks = state.tasks.clone();

    view! {
        <div class:done={ task.done } class="task-row">
            <span class="task-text">{ task.title.clone() }</span>
            <button on:click={ move |_| {
                let current: Vec<Task> = tasks.get();
                let updated: Vec<Task> = current.into_iter().map(|mut t| {
                    if t.id == id { t.done = !t.done; }
                    t
                }).collect();
                tasks.with_mut(|v| *v = updated);
            }}>
                { if task.done { "Mark open" } else { "Done" } }
            </button>
        </div>
    }
}
