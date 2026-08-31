# Zero-Clone Patterns & Ownership

In Rust WebAssembly, managing closure captures and reference clones efficiently leads to clean, maintainable code.

---

## 1. Using `RwSignal` to Avoid Split Cloning

Instead of maintaining separate read and write handles:

```rust
// ❌ Split signal requires cloning both read and write handles
let (count, set_count) = create_signal(0);
let count_for_read = count.clone();
let set_count_for_write = set_count.clone();

// ✅ RwSignal requires only a single handle clone
let count = signal(0);
let count_for_click = count.clone();
```

---

## 2. Leveraging Two-Way Binding (`bind:value`)

Eliminate manual event target downcasting:

```rust
// ❌ Manual event extraction with web_sys casting
let input = signal(String::new());
let input_writer = input.clone();
view! {
    <input on:input={ move |e| {
        let val = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
        input_writer.set(val);
    }} />
}

// ✅ Two-way binding sugar (zero manual clone, zero event boilerplate)
let input = signal(String::new());
view! {
    <input bind:value={ input } />
}
```

---

## 3. Pulling State from Context in Child Components

Instead of cloning models and passing them through props in deep component trees or keyed list loops:

```rust
// Pull shared state inside the component using use_context
#[component]
fn TaskItem(task: Task) {
    let state = use_context::<AppState>().expect("AppState in context");
    let id = task.id;
    let state_for_toggle = state.clone();

    view! {
        <li class:done={ task.completed }>
            <span>{ task.title.clone() }</span>
            <button on:click={ move |_| state_for_toggle.toggle_task(id) }>
                "Toggle"
            </button>
        </li>
    }
}
```
