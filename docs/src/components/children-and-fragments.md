# Children & Document Fragments

---

## 1. Document Fragments (`<> ... </>`)

Document Fragments allow you to group a list of sibling elements without introducing an extra wrapper `<div>` into the browser DOM:

```rust
view! {
    <>
        <li>"First Item"</li>
        <li>"Second Item"</li>
        <li>"Third Item"</li>
    </>
}
```

When appended to a parent element in the browser, the fragment unpacks its children directly into the target container.

---

## 2. Component Composition with Children

When an uppercase component tag contains child markup, the child nodes are passed directly to the component:

```rust
#[component]
fn Card(header: String) {
    view! {
        <div class="card">
            <header><h3>{ header }</h3></header>
        </div>
    }
}
```

---

## 3. Returning Multiple Root Nodes

Because `DomNode::fragment()` returns a valid `DomNode`, your component functions can return multiple root nodes directly using fragment syntax `<> ... </>`.
