# The `DomNode` API

`DomNode` is Velo's lightweight wrapper around native `web_sys::Node` instances.

---

## 1. Node Constructors

```rust
use velo_dom::DomNode;

// Create standard element
let div = DomNode::element("div");

// Create document fragment
let frag = DomNode::fragment();

// Create static text node
let text = DomNode::text("Static string");

// Create reactive text node
let (name, _) = create_signal("Ada".to_string());
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
let (theme, _) = create_signal("dark".to_string());
let t = theme.clone();
btn.reactive_attribute("class", move || t.get());

// Reactive class toggle
let (is_active, _) = create_signal(true);
let a = is_active.clone();
btn.toggle_class("active", move || a.get());

// Reactive CSS style
let (color, _) = create_signal("red".to_string());
let c = color.clone();
btn.reactive_style("color", move || c.get());
```
