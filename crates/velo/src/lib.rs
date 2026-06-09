// Re-export the core reactive primitives
pub mod velo_core {
    pub use velo_core::*;
}

// Re-export the DOM rendering layer
pub mod velo_dom {
    pub use velo_dom::*;
}

// Re-export the pattern matching router structures
pub mod velo_router {
    pub use velo_router::*;
}

// Re-export the procedural macro globally as the foundational view compiler!
pub use velo_macro::view;

// Create a unified prelude for effortless application imports
pub mod prelude {
    pub use crate::velo_core::{create_effect, Signal};
    pub use crate::velo_dom::DomNode;
    pub use crate::velo_router::{Link, Route, Router};
    pub use crate::view;
}
