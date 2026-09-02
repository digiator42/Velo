# Event Handling

Events are bound in the `view!` macro using the `on:<event_name>` attribute syntax.

---

## 1. Basic Event Binding

Pass a closure taking a `web_sys::Event` parameter:

```rust
let count = signal(0);
let count_for_click = count.clone();

view! {
    <button on:click={ move |_event| {
        count_for_click.update(|c| *c += 1);
    }}>
        "Clicked " { count } " times"
    </button>
}
```

---

## 2. Common Event Types

You can listen to any native browser DOM event:
* `on:click`
* `on:input`
* `on:change`
* `on:submit`
* `on:keydown`, `on:keyup`
* `on:mouseenter`, `on:mouseleave`
* `on:focus`, `on:blur`

---

## 3. Accessing the Event Object

To inspect the event, bind it to a parameter:

```rust
view! {
    <button on:click={ move |e| {
        web_sys::Event::type_(&e);
    }}>
        "Inspect"
    </button>
}
```

---

## 4. Form Submit Sugar (`on:submit`)

**`on:submit` calls `event.prevent_default()` automatically**, so your WASM app
handles the submit instead of the browser reloading or navigating. You can
write a zero-argument handler and skip the boilerplate entirely:

```rust
view! {
    <form on:submit={ () => {
        // NO manual prevent_default() needed — the sugar does it for you.
        tasks.push(Task { title: input.get(), done: false });
        input.set(String::new());  // controlled reset via bind:value
    }}>
        <input type="text" bind:value={ input } />
        <button type="submit">"Add"</button>
    </form>
}
```

If you also need the event itself, bind it as a parameter — `prevent_default`
is still applied for you:

```rust
view! {
    <form on:submit={ move |e| {
        web_sys::console::log_1(&"Submitted".into());
        // e.prevent_default() is redundant here — already handled.
    }}> { /* ... */ } </form>
}
```

This pairs with `bind:value`/`bind:checked` (see
[Two-Way Form Bindings](two-way-binding.md)) to build controlled forms with no
event-parsing boilerplate.

---

## 5. Automatic Listener Teardown

Event listeners attached through `DomNode::on` or the `view!` macro are owned by the `DomNode`. When the node is removed from the DOM and dropped, all attached closures and listeners are automatically detached from the browser element, preventing memory leaks!
