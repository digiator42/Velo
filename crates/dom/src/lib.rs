use velo_core::{create_effect, SignalVec};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node};

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

// Case A: The expression inside the braces returns a single DomNode
impl RenderDynamic for DomNode {
    fn render_dynamic(self) -> DomNode {
        self
    }
}

// Case B: The expression returns a collected Vector of nodes from an iterator
impl RenderDynamic for Vec<DomNode> {
    fn render_dynamic(self) -> DomNode {
        let frag = DomNode::fragment();
        for node in self {
            frag.append(&node);
        }
        frag
    }
}

// Case C: Macro to implement RenderDynamic explicitly for common types.
// This satisfies the compiler completely because Vec<DomNode> is none of these!
macro_rules! impl_render_for_primitives {
    ($($t:ty),*) => {
        $(
            impl RenderDynamic for $t {
                fn render_dynamic(self) -> DomNode {
                    DomNode::text(&format!("{}", self))
                }
            }
        )*
    };
}

// Tell the framework how to dynamically render text, numbers, and booleans inline
impl_render_for_primitives!(
    String, &str, bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

// Any `RenderDynamic` type is a plain (non-signal) view value. `PlainViewValue`
// is a marker implemented only here, never for signals, so the `ViewValue`
// blanket below doesn't overlap with the `Signal`/`ReadSignal` impls.
impl<T: RenderDynamic + 'static> PlainViewValue for T {}

/// Trait powering the `view!` macro's automatic signal unwrapping.
///
/// `view! { { count } }` and `name={ signal }` wrap the expression in
/// `signal_value!(..)`, which calls [`ViewValue::view_value`]. Signal types
/// unwrap to their inner value (and subscribe the running effect); plain
/// `RenderDynamic` values pass through unchanged. Takes `&self` so the source
/// handle is borrowed, not moved (handles may be reused in other closures).
pub trait ViewValue {
    type Out;
    fn view_value(&self) -> Self::Out;
}

impl<T: Clone + 'static> ViewValue for velo_core::ReadSignal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for velo_core::Signal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for velo_core::Memo<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> ViewValue for velo_core::RwSignal<T> {
    type Out = T;
    fn view_value(&self) -> T {
        self.get()
    }
}

// Marker trait so plain RenderDynamic values can get a ViewValue blanket
// impl without overlapping the `Signal`/`ReadSignal` impls above.
pub trait PlainViewValue {}

// Gated on `PlainViewValue` so it never overlaps with the `Signal`/`ReadSignal`
// impls above (those types intentionally do not implement `PlainViewValue`).
impl<T: PlainViewValue + Clone + 'static> ViewValue for T {
    type Out = T;
    fn view_value(&self) -> T {
        self.clone()
    }
}

/// Helper used by the `view!` macro: read a reactive value (or pass through a
/// plain value) inside the current effect so it becomes a tracking dependency.
#[macro_export]
macro_rules! signal_value {
    ($expr:expr) => {{
        $crate::ViewValue::view_value(&$expr)
    }};
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

    /// Creates an empty DocumentFragment node to act as a zero-wrapper container for siblings.
    /// When appended to a parent, the browser automatically unpacks its children directly.
    pub fn fragment() -> Self {
        let frag = document().create_document_fragment();
        Self {
            raw_node: frag.into(),
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

    /// Toggles a single class name on/off reactively based on a boolean signal.
    pub fn toggle_class<F>(&self, class_name: &str, mut is_on: F)
    where
        F: FnMut() -> bool + 'static,
    {
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only toggle classes on element nodes");
        let class_name = class_name.to_string();

        create_effect(move || {
            let on = is_on();
            if on {
                let _ = el.class_list().add_1(&class_name);
            } else {
                let _ = el.class_list().remove_1(&class_name);
            }
        });
    }

    /// Binds a CSS inline style property reactively to a string value. Multiple
    /// `reactive_style` calls on the same element are merged (each keeps its own
    /// property) by accumulating into the element's `style` attribute.
    pub fn reactive_style<F>(&self, property: &str, mut f: F)
    where
        F: FnMut() -> String + 'static,
    {
        let el: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only bind styles to element nodes");
        let property = property.to_string();

        // Accumulate inline style properties so concurrent bindings don't clobber.
        let styles: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
        let styles_c = std::rc::Rc::clone(&styles);

        create_effect(move || {
            let value = f();
            styles_c.borrow_mut().insert(property.clone(), value);

            let css: String = styles_c
                .borrow()
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = el.set_attribute("style", &css);
        });
    }

    /// Mounts a keyed, fine-grained reactive list. On each change the reconciler
    /// inserts/removes/moves real DOM nodes to match the new keyed items, instead
    /// of blowing away the whole subtree. `key` extracts the stable key; `render`
    /// builds the `DomNode` for one item.
    pub fn render_signal_vec<T, K, FKey, FRender>(
        &self,
        list: &SignalVec<T>,
        key: FKey,
        render: FRender,
    ) where
        T: Clone + 'static,
        K: Eq + std::hash::Hash + Clone + 'static,
        FKey: Fn(&T) -> K + 'static,
        FRender: Fn(&T) -> DomNode + 'static + Clone,
    {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let container: Element = self
            .raw_node
            .clone()
            .dyn_into()
            .expect("Velo: Can only render a list into an element node");

        // Node map keyed by the item's stable key, plus an ordered list of keys
        // so we can detect moves/restores cheaply.
        let nodes: Rc<RefCell<HashMap<K, DomNode>>> = Rc::new(RefCell::new(HashMap::new()));
        let order: Rc<RefCell<Vec<K>>> = Rc::new(RefCell::new(Vec::new()));

        let list = list.clone();
        let nodes_c = Rc::clone(&nodes);
        let order_c = Rc::clone(&order);

        create_effect(move || {
            let items: Vec<T> = list.get();
            let new_keys: Vec<K> = items.iter().map(|it| key(it)).collect();

            // Remove nodes whose key disappeared.
            let live: std::collections::HashSet<K> = new_keys.iter().cloned().collect();
            let stale: Vec<K> = order_c
                .borrow()
                .iter()
                .filter(|k| !live.contains(k))
                .cloned()
                .collect();
            for k in stale {
                if let Some(node) = nodes_c.borrow_mut().remove(&k) {
                    let _ = container.remove_child(&node.raw_node);
                }
            }

            // Build a lookup of new items by key for quick render.
            let mut by_key: HashMap<K, DomNode> = HashMap::new();
            for it in &items {
                let k = key(it);
                let node = render(it);
                by_key.insert(k, node);
            }

            // Reconcile against previous order: insert/move each item before the
            // node that precedes it in the previous layout (or append at end).
            let prev = order_c.borrow().clone();
            for (idx, k) in new_keys.iter().enumerate() {
                let node = match nodes_c.borrow_mut().get(k) {
                    Some(existing) => existing.clone(),
                    None => {
                        let n = by_key.remove(k).expect("rendered node for key");
                        nodes_c.borrow_mut().insert(k.clone(), n.clone());
                        n
                    }
                };

                let reference = if idx + 1 < new_keys.len() {
                    nodes_c
                        .borrow()
                        .get(&new_keys[idx + 1])
                        .map(|n| n.raw_node.clone())
                } else {
                    None
                };

                // Only re-insert if its position actually changed.
                let needs_move = prev.get(idx) != Some(k);
                if needs_move {
                    let _ = container.insert_before(&node.raw_node, reference.as_ref());
                }
            }

            *order_c.borrow_mut() = new_keys;
        });
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

/// Mounts the root framework application element directly to a target DOM
/// container ID.
///
/// **Deprecated:** prefer [`mount`] (body) or [`mount_at`] (explicit node).
/// `mount_to_id` appends as a child (creating a wrapper) and cannot be
/// unmounted via a returned handle.
#[deprecated(note = "prefer mount()/mount_at(); see Velo docs §8")]
pub fn mount_to_id(id: &str, root_node: DomNode) {
    let container = document()
        .get_element_by_id(id)
        .expect("Velo: Mount targets specified container element ID could not be located");

    container
        .append_child(&root_node.raw_node)
        .expect("Velo: Failed to mount root node allocation to layout tree container target");
}

// ---------------------------------------------------------------------------
// Modern Mounting API  (§8): mount(), mount_at(), RootHandle
// ---------------------------------------------------------------------------

/// A handle to a mounted Velo root. Dropping it unmounts the app from the
/// DOM (removes the root node from its parent). Calling `.unmount()` does
/// the same explicitly and is idempotent.
///
/// Returned by [`mount`] and [`mount_at`] so the caller can tear down the
/// entire app (useful for testing, remounting, and hot-reload teardown).
#[derive(Clone)]
pub struct RootHandle {
    root: DomNode,
}

impl RootHandle {
    /// Remove the root node(s) from the DOM. Safe to call multiple times.
    pub fn unmount(self) {
        if let Some(parent) = self.root.raw_node.parent_node() {
            let _ = parent.remove_child(&self.root.raw_node);
        }
    }
}

impl Drop for RootHandle {
    fn drop(&mut self) {
        // Dispose any root-level effects here once effect-to-node tracking is
        // in place. For now the DomNode Drop removes the node itself.
        if let Some(parent) = self.root.raw_node.parent_node() {
            let _ = parent.remove_child(&self.root.raw_node);
        }
    }
}

/// Convenience: mount a `DomNode` tree into `document.body()` as a fragment
/// root (no wrapper element). Returns a `RootHandle` that can be dropped or
/// `.unmount()`ed to tear the app down.
///
/// ```ignore
/// let app = view! { <div class="page">...</div> };
/// let handle = velo_dom::mount(app);
/// // later: handle.unmount();
/// ```
pub fn mount(root: DomNode) -> RootHandle {
    let body = document()
        .body()
        .expect("Velo: document has no body — are you running in a browser?");
    mount_at(&body, root)
}

/// Mount a `DomNode` tree into a specific DOM `Node` as a fragment root.
/// The root is **appended** into the target as a child (fragment roots
/// unpack automatically, so there is no wrapper element).
///
/// Returns a `RootHandle` for explicit unmount / Drop-based teardown.
///
/// ```ignore
/// let app = view! { <div class="page">...</div> };
/// let div = document().get_element_by_id("app").unwrap();
/// let handle = velo_dom::mount_at(&div, app);
/// ```
pub fn mount_at(target: &web_sys::Node, root: DomNode) -> RootHandle {
    target
        .append_child(&root.raw_node)
        .expect("Velo: Failed to mount root node into target");
    RootHandle { root }
}

/// Mounts the root framework application element directly to a target DOM
/// container ID.
///
/// **Deprecated:** prefer [`mount`] (body) or [`mount_at`] (explicit node).
/// `mount_to_id` appends as a child (creating a wrapper) and cannot be
/// unmounted via a returned handle.
#[deprecated(note = "prefer mount()/mount_at(); see Velo docs §8")]
pub fn mount_to_id_deprecated(id: &str, root_node: DomNode) {
    let container = document()
        .get_element_by_id(id)
        .expect("Velo: Mount targets specified container element ID could not be located");

    container
        .append_child(&root_node.raw_node)
        .expect("Velo: Failed to mount root node allocation to layout tree container target");
}


