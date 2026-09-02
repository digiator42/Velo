use velo::prelude::*;

/// A page that "fails while rendering". `app!` wraps every page in an error
/// boundary; raising a boundary fault (the wasm-safe equivalent of an injected
/// panic — a real `panic!` aborts a `wasm32` instance, see `error_boundary`)
/// renders the nearest `error.rs` fallback while the rest of the app lives.
#[page]
pub fn page() -> DomNode {
    velo::boundary_fault("boom from the broken page")
}