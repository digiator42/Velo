use velo_core::{create_effect, Signal};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node, Text};

/// Helper to easily access the global window document instance
pub fn document() -> Document {
    web_sys::window()
        .expect("Velo: No global window found. Are you running in a browser environment?")
        .document()
        .expect("Velo: Window should contain a valid document.")
}

pub trait RenderDynamic {
    fn render_dynamic(self) -> DomNode;
}

// Case A: The expression inside the braces returns a DomNode (like a sub-component)
impl RenderDynamic for DomNode {
    fn render_dynamic(self) -> DomNode {
        self
    }
}

// Case B: The expression returns something that can be displayed (like an i32 or String)
impl<T: std::fmt::Display + 'static> RenderDynamic for T {
    fn render_dynamic(self) -> DomNode {
        DomNode::text(&format!("{}", self))
    }
}

/// A wrapper around a real native browser DOM element
#[derive(Clone)]
pub struct DomNode {
    pub raw_node: Node,
}

impl DomNode {
    /// Creates a standard HTML element container (e.g., "div", "button", "h1")
    pub fn element(tag: &str) -> Self {
        let el = document()
            .create_element(tag)
            .expect("Velo: Failed to create DOM element tag");
        Self {
            raw_node: el.into(),
        }
    }

    /// Creates a static, un-changing text node
    pub fn text(content: &str) -> Self {
        let txt = document().create_text_node(content);
        Self {
            raw_node: txt.into(),
        }
    }

    /// Sets the text content of this node directly, replacing any existing children.
    pub fn set_text(&self, content: &str) {
        self.raw_node.set_text_content(Some(content));
    }

    /// Creates a modern, fine-grained reactive text node.
    /// The node binds itself to a reactive Signal closure and updates surgically.
    pub fn reactive_text<F>(mut f: F) -> Self
    where
        F: FnMut() -> String + 'static,
    {
        let txt_node = document().create_text_node("");
        let current_node = txt_node.clone();

        // Establish the tracking wrapper loop
        create_effect(move || {
            let evaluated_string = f();
            current_node.set_node_value(Some(&evaluated_string));
        });

        Self {
            raw_node: txt_node.into(),
        }
    }

    /// Appends a child DomNode into this node's layout structure
    pub fn append(&self, child: &DomNode) {
        self.raw_node
            .append_child(&child.raw_node)
            .expect("Velo: Failed to append child node layout hook");
    }

    /// Binds an attribute (like "class" or "id") directly to a reactive Signal
    pub fn reactive_attribute<F>(&self, name: &str, mut f: F)
    where
        F: FnMut() -> String + 'static,
    {
        // We cast the generic Node back to an Element to mutate attributes safely
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only bind attributes to element nodes");
        let attr_name = name.to_string();

        create_effect(move || {
            let value = f();
            el.set_attribute(&attr_name, &value)
                .expect("Velo: Failed to update element node attribute");
        });
    }

    /// Attaches an event listener directly to the browser element wrapper
    pub fn on<F>(&self, event_name: &str, mut handler: F)
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            handler(e);
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.raw_node
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
            .expect("Velo: Failed to attach event listener callback");

        // Prevent Rust from cleaning up the closure allocation context immediately
        closure.forget();
    }

    /// Explicitly handles components, sub-views, or existing DomNodes passed into braces
    pub fn from_node(node: DomNode) -> Self {
        node
    }

    /// Explicitly handles dynamic closures passed into braces for surgical text updates
    pub fn from_closure<F>(mut f: F) -> Self
    where
        F: FnMut() -> String + 'static,
    {
        Self::reactive_text(move || f())
    }

    /// Accepts a closure from the macro, evaluates it inside an effect loop,
    /// and resolves whether it's rendering a component or text dynamically!
    pub fn render_expression<F, R>(mut f: F) -> Self
    where
        F: FnMut() -> R + 'static,
        R: RenderDynamic + 'static,
    {
        // Change "div" to "span" to make the expression wrapper an inline element!
        let container = document()
            .create_element("span")
            .expect("Velo: Failed to create expression wrapper block");
        container
            .set_attribute("class", "velo-expression-wrapper")
            .unwrap();

        let container_raw = container.clone();
        let mut f_clone = move || f();

        velo_core::create_effect(move || {
            let val: R = f_clone();
            let resolved_node = val.render_dynamic();

            container_raw.set_text_content(None);

            container_raw
                .append_child(&resolved_node.raw_node)
                .expect("Velo: Failed to append dynamic expression variant");
        });

        Self {
            raw_node: container.into(),
        }
    }
}

/// Mounts the root framework application element directly to a target DOM container ID
pub fn mount_to_id(id: &str, root_node: DomNode) {
    let container = document()
        .get_element_by_id(id)
        .expect("Velo: Mount targets specified container element ID could not be located");

    container
        .append_child(&root_node.raw_node)
        .expect("Velo: Failed to mount root node allocation to layout tree container target");
}
