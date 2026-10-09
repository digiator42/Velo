# The `DomNode` API

`DomNode` is Velo's lightweight wrapper around native `web_sys::Node` instances.

---

## 1. Node Constructors

```rust
use velo::DomNode;

// Create standard element
let div = DomNode::element("div");

// Create document fragment
let frag = DomNode::fragment();

// Create static text node
let text = DomNode::text("Static string");

// Create reactive text node
let name = signal!("Ada".to_string());
let reactive_name = name.clone();
let text_node = DomNode::reactive_text(move || reactive_name.get());
```

---

## 2. Tree Mutation & Structure

```rust
let parent = DomNode::element("div");
let child = DomNode::element("p");

// Append child (moves effect handles into parent)
parent.append(&child);

// Set static text content directly
child.set_text("Updated paragraph text");
```

---

## 3. Native Bindings & Listeners

```rust
let btn = DomNode::element("button");

// Bind event listener (automatically detached on drop)
btn.on("click", move |_event| {
    web_sys::console::log_1(&"Clicked".into());
});

// Reactive attribute binding
let theme = signal!("dark".to_string());
let t = theme.clone();
btn.reactive_attribute("class", move || t.get());

// Reactive class toggle
let is_active = signal!(true);
let a = is_active.clone();
btn.toggle_class("active", move || a.get());

// Reactive CSS style
let color = signal!("red".to_string());
let c = color.clone();
btn.reactive_style("color", move || c.get());
```

---

## 4. Static Attributes (no effect)

Literal `view!` attributes call these directly — no effect, no subscription:

```rust
// Set/remove an attribute imperatively
div.set_attr("data-state", "open");
div.remove_attr("data-state");
```

---

## 5. Resource Ownership

```rust
// Retain an effect handle for this node's lifetime (non-leaking
// alternative to mem::forget); disposed when the node drops.
btn.retain_effect(handle);
btn.retain_effects(vec![handle_a, handle_b]);

// Stop owning a child's resources after removing it, so its
// effects/listeners are disposed instead of lingering.
parent.release(&child);
```

See [Effect Lifecycle & Resource Cleanup](effect-cleanup.md) for the full
ownership model (absorption, scopes, and disposal rules).
