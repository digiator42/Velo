# Example: Task Manager (Todo App)

A multi-page task management application featuring client-side routing, keyed reactive lists, two-way form bindings, and global context state.

---

## 1. Application Models & Context

```rust
use velo::prelude::*;

#[derive(Clone)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub tasks: SignalVec<Task>,
}
```

---

## 2. Components

```rust
#[allow(non_snake_case)]
#[component]
pub fn TaskRow(task: Task) {
    let state = use_context::<AppState>().expect("AppState in context");
    let id = task.id;
    let tasks = state.tasks.clone();

    view! {
        <div class:done={ task.completed } class="task-row">
            <span class="task-title">{ task.title.clone() }</span>
            <button on:click={ move |_| {
                let current = tasks.get();
                let updated: Vec<Task> = current.into_iter().map(|mut t| {
                    if t.id == id { t.completed = !t.completed; }
                    t
                }).collect();
                tasks.with_mut(|v| *v = updated);
            }}>
                { if task.completed { "Undo" } else { "Complete" } }
            </button>
        </div>
    }
}
```

---

## 3. Pages & Routing

With `app!` file routing the `/tasks` page lives at `src/app/tasks/page.rs` and
is annotated `#[page]`. The macro builds the route table and a typed
`paths::*` helper for it:

```rust
// src/app/tasks/page.rs
use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    let state = use_context::<AppState>().expect("AppState in context");
    let input = signal!(String::new());

    let state_add = state.clone();
    let input_add = input.clone();

    let total = memo({ let t = state.tasks.clone(); move || t.get().len() as u32 });

    view! {
        <div class="page">
            <h2>"Task Manager"</h2>

            <div class="input-bar">
                <input type="text" placeholder="New task..." bind:value={ input } />
                <button on:click={ move |_| {
                    let text = input_add.get().trim().to_string();
                    if !text.is_empty() {
                        let next_id = state_add.tasks.get().iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        state_add.tasks.push(Task { id: next_id, title: text, completed: false });
                        input_add.set(String::new());
                    }
                }}>"Add"</button>
            </div>

            <div class="stats">
                <span>"Total items: " { total }</span>
            </div>

            <div class="task-list">
                {
                    for t in state.tasks key = |t: &Task| t.id {
                        <TaskRow task={ t.clone() } />
                    }
                }
            </div>
        </div>
    }
}
```

The app root wires everything together with `velo::app!()` and a `<Router>`:

```rust
// src/lib.rs
use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();

#[wasm_bindgen(start)]
pub fn main() { run_app(); }

pub fn run_app() {
    let shell = view! {
        <div id="app">
            <Router routes={ velo_app::routes() } />
        </div>
    };
    mount(shell);
}
```

> The real `examples/todo-app` predates file routing and uses a manual
> `Vec<Route>` table. Both styles work; the `app!`/`#[page]` form is the
> recommended one and is shown here.
