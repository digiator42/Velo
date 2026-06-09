use dom::{document, DomNode};
use std::cell::RefCell;
use std::rc::Rc;
use velo_core::{create_effect, Signal};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    // Global tracking signal for the current URL path string
    pub static CURRENT_PATH: Signal<String> = Signal::new(
        web_sys::window()
            .expect("Velo Router: No window found")
            .location()
            .pathname()
            .expect("Velo Router: Failed to read pathname")
    );
}

/// Programmatically updates the browser URL and alerts the active Route Signal
pub fn navigate_to(path: &str) {
    let window = web_sys::window().expect("Velo Router: No window found");
    let history = window
        .history()
        .expect("Velo Router: Accessing history failed");

    // Update browser history state without refreshing the page
    history
        .push_state_with_url(&JsValue::NULL, "", Some(path))
        .expect("Velo Router: push_state failed");

    // Inform our fine-grained reactive loop about the location update
    CURRENT_PATH.with(|path_signal| {
        path_signal.set(path.to_string());
    });
}

/// Initializes global browser listeners to intercept browser back/forward buttons
pub fn init_router_listeners() {
    let window = web_sys::window().expect("Velo Router: No window found");

    let on_popstate = Closure::wrap(Box::new(move |_e: web_sys::PopStateEvent| {
        let current_path_str = web_sys::window().unwrap().location().pathname().unwrap();

        CURRENT_PATH.with(|path_signal| {
            path_signal.set(current_path_str);
        });
    }) as Box<dyn FnMut(web_sys::PopStateEvent)>);

    window
        .add_event_listener_with_callback("popstate", on_popstate.as_ref().unchecked_ref())
        .expect("Velo Router: Failed to bind popstate listener");

    on_popstate.forget();
}

pub struct Route {
    pub path: &'static str,
    pub component: fn() -> DomNode,
}

/// Router structural component. Listens to path changes and morphs the core view.
pub struct FRouter;

impl FRouter {
    pub fn new<F>(mut routes_matcher: F) -> DomNode
    where
        F: FnMut(&str) -> DomNode + 'static,
    {
        // Setup global event interception right away
        init_router_listeners();

        // Create a persistent placeholder element where pages dynamically swap out
        let view_wrapper = DomNode::element("div");
        view_wrapper.reactive_attribute("class", || "velo-router-viewport".to_string());

        let current_child: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));

        let wrapper_raw = view_wrapper.raw_node.clone();
        let child_tracker = Rc::clone(&current_child);

        // This effect executes SURGICALLY only when the URL changes
        create_effect(move || {
            let path = CURRENT_PATH.with(|p| p.get());

            // Clean up previous view node from browser memory layout tree if it exists
            if let Some(old_node) = child_tracker.borrow().as_ref() {
                let _ = wrapper_raw.remove_child(&old_node.raw_node);
            }

            // Build the new page node matched via the developer's closure
            let new_page_node = routes_matcher(&path);

            wrapper_raw
                .append_child(&new_page_node.raw_node)
                .expect("Velo Router: Failed to append target route content");

            // Store reference to clean up on the next navigation cycle
            *child_tracker.borrow_mut() = Some(new_page_node);
        });

        view_wrapper
    }
}

// Ergonomic Router component for clean macro nesting: <Router>{ |path| ... }</Router>
#[allow(non_snake_case)]
pub fn Router(routes: Vec<Route>) -> DomNode {
    init_router_listeners();

    let view_wrapper = DomNode::element("div");
    view_wrapper.reactive_attribute("class", || "velo-router-viewport".to_string());

    // Initialize as a clean Option<DomNode> using an explicit type annotation!
    let current_child: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));
    
    let wrapper_raw = view_wrapper.raw_node.clone();
    let child_tracker = Rc::clone(&current_child);

    create_effect(move || {
        let current_path = CURRENT_PATH.with(|p| p.get());

        // Now child_tracker.borrow().as_ref() correctly yields a &DomNode!
        if let Some(old_node) = child_tracker.borrow().as_ref() {
            let _ = wrapper_raw.remove_child(&old_node.raw_node);
        }

        let matched_component = routes
            .iter()
            .find(|r| r.path == current_path)
            .map(|r| (r.component)())
            .unwrap_or_else(|| {
                let fallback = DomNode::element("h1");
                fallback.append(&DomNode::text("404 - Page Not Found"));
                fallback
            });

        wrapper_raw
            .append_child(&matched_component.raw_node)
            .expect("Velo Router: Failed to append target route content");

        // Types match perfectly here now!
        *child_tracker.borrow_mut() = Some(matched_component);
    });

    view_wrapper
}

/// Ergonomic Link component that allows clean macro nesting: <Link to="...">Children</Link>
#[allow(non_snake_case)]
pub fn Link(to: &'static str, label: &'static str) -> DomNode {
    let anchor = DomNode::element("a");
    anchor.reactive_attribute("href", move || to.to_string());

    // Create a text node from your implementation and attach it
    let text_content = DomNode::text(label);
    anchor.append(&text_content);

    anchor.on("click", move |event| {
        event.prevent_default();
        navigate_to(to);
    });

    anchor
}
