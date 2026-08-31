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

To inspect the event or prevent default browser behavior:

```rust
view! {
    <form on:submit={ move |e| {
        e.prevent_default(); // Prevent page reload
        web_sys::console::log_1(&"Form submitted!".into());
    }}>
        <button type="submit">"Submit"</button>
    </form>
}
```

---

## 4. Automatic Listener Teardown

Event listeners attached through `DomNode::on` or the `view!` macro are owned by the `DomNode`. When the node is removed from the DOM and dropped, all attached closures and listeners are automatically detached from the browser element, preventing memory leaks!
