// 1. Re-export the underlying crates as public top-level modules.
//    This allows explicit pathways like `velo::velo_dom::DomNode` to resolve perfectly.
pub use velo_core as core;
pub use velo_dom as dom;
pub use velo_macro as macro_internal;
pub use velo_router as router;

// Make velo_dom available as a crate path so the view! macro's generated
// `velo_dom::` references resolve even when consumers only depend on `velo`.
pub use velo_dom;

// 2. Re-export the foundational items at the root level of the crate.
//    This fixes the macro generation path error for `velo::create_effect`.
pub use velo_core::{
    batch,
    create_effect,
    create_effect_with_cleanup,
    create_memo,
    create_signal,
    memo,
    provide_context,
    signal,
    signal_vec,
    use_context,
    with_context,
    ReadSignal,
    RwSignal,
    Signal,
    SignalVec,
    WriteSignal,
};
pub use velo_dom::{signal_value, ViewValue};
pub use velo_macro::{component, view};

#[doc(hidden)]
pub use velo_dom::PlainViewValue;

/// The Unified Framework Prelude
/// Bringing this into scope via `use velo::prelude::*;` fully seeds your application files
/// with reactivity primitives, DOM nodes, components, routers, and the view! macro.
pub mod prelude {
    // Re-export core reactive primitives
    pub use velo_core::{
        batch,
        create_effect,
        create_effect_with_cleanup,
        create_memo,
        create_signal,
        memo,
        provide_context,
        signal,
        signal_vec,
        use_context,
        with_context,
        ReadSignal,
        RwSignal,
        Signal,
        SignalVec,
        WriteSignal,
    };
    // Re-export the signal-unwrapping machinery used by the view! macro
    #[doc(hidden)]
    pub use velo_dom::PlainViewValue;
    pub use velo_dom::{signal_value, ViewValue};

    // Re-export primary DOM manipulation types and helpers
    pub use velo_dom::{
        document, mount, mount_at, mount_to_id, DomNode, RenderDynamic, RootHandle,
    };

    // Re-export router structures
    pub use velo_router::{Link, Route, Router};

    // Re-export the view! + #[component] procedural macros
    pub use velo_macro::{component, view, routes};

    // Bring the nested sub-crate identifiers directly inside the prelude namespace scope.
    // This allows references like `velo_dom::DomNode` to be completely understood
    // without needing separate, manual imports!
    pub use crate::core;
    pub use crate::router;
    pub use crate::dom;
}
