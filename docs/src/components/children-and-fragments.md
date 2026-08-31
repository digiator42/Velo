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

When an uppercase component tag contains child markup, the child nodes are routed into a
`children: Vec<DomNode>` parameter and passed to the component. This enables natural
`<Panel>{ .. }</Panel>` composition:

```rust
#[component]
fn Panel(header: String, children: Vec<DomNode>) {
    view! {
        <div class="panel">
            <header><h3>{ header }</h3></header>
            <div class="panel-body">{ children }</div>
        </div>
    }
}

// In a parent view — nested markup becomes the `children` prop:
view! {
    <Panel header="Account".into()>
        <p>"Anything you put here is the Panel's children."</p>
        <button on:click={ move |_| { /* ... */ } }>"Save"</button>
    </Panel>
}
```

A `children={ expr }` attribute is also honored and takes precedence over nested markup.

---

## 3. Returning Multiple Root Nodes

Because `DomNode::fragment()` returns a valid `DomNode`, your component functions can return multiple root nodes directly using fragment syntax `<> ... </>`.
