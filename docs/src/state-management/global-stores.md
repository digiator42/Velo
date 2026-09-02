# Global Application State Stores

For larger applications with complex domains, grouping related signals and collections into a centralized state model makes state transitions predictable and easy to test.

---

## 1. Structuring an App State Store

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
    pub filter: RwSignal<String>,
    pub search_query: RwSignal<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tasks: signal_vec(Vec::new()),
            filter: signal("all".to_string()),
            search_query: signal(String::new()),
        }
    }

    pub fn add_task(&self, title: String) {
        let next_id = self.tasks.get().iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tasks.push(Task {
            id: next_id,
            title,
            completed: false,
        });
    }

    pub fn toggle_task(&self, id: u32) {
        let current = self.tasks.get();
        let updated: Vec<Task> = current.into_iter().map(|mut t| {
            if t.id == id {
                t.completed = !t.completed;
            }
            t
        }).collect();
        self.tasks.with_mut(|vec| *vec = updated);
    }
}
```

---

## 2. Providing the Store Globally

```rust
pub fn run_app() {
    provide_context(AppState::new());
    mount(root_view());
}
```

---

## 3. Consuming Store Methods in Components

```rust
#[component]
fn AddTaskBar() {
    let state = use_context::<AppState>().expect("AppState in context");
    let input = signal(String::new());

    let state_for_add = state.clone();
    let input_for_add = input.clone();

    view! {
        <div class="add-task-bar">
            <input type="text" placeholder="What needs doing?" bind:value={ input } />
            <button on:click={ move |_| {
                let text = input_for_add.get().trim().to_string();
                if !text.is_empty() {
                    state_for_add.add_task(text);
                    input_for_add.set(String::new());
                }
            }}>
                "Add Task"
            </button>
        </div>
    }
}
```
