// 1. Re-export the underlying crates as public top-level modules.
//    This allows explicit pathways like `velo::velo_dom::DomNode` to resolve perfectly.
pub use velo_core as core;
pub use velo_dom;
pub use velo_macro as macro_internal;
pub use velo_router as router;

// 2. Re-export the foundational items at the root level of the crate.
//    This fixes the macro generation path error for `velo::create_effect`.
pub use velo_core::create_effect;
pub use velo_macro::view;

/// The Unified Framework Prelude
/// Bringing this into scope via `use velo::prelude::*;` fully seeds your application files
/// with reactivity primitives, DOM nodes, components, routers, and the view! macro.
pub mod prelude {
    // Re-export core reactive primitives
    pub use velo_core::{create_effect, Signal};

    // Re-export primary DOM manipulation types and helpers
    pub use velo_dom::{document, DomNode, RenderDynamic};

    // Re-export router structures
    pub use velo_router::{
        // Add your standard routing structs here (e.g., Router, Route, Link)
        Link,
        Route,
        Router,
    };

    // Re-export the main view procedural macro
    pub use velo_macro::view;

    // Bring the nested sub-crate identifiers directly inside the prelude namespace scope.
    // This allows references like `velo_dom::DomNode` to be completely understood
    // without needing separate, manual imports!
    pub use crate::core;
    pub use crate::router;
    pub use crate::velo_dom;
}
