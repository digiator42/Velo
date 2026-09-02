use velo::prelude::*;

/// Dashboard at `/`: a delayed fake "fetch" feeds `<Suspense>`, which shows a
/// fallback while loading and swaps in the resolved value when the resource
/// flips. The link below also demonstrates per-route `loading.rs` (global, in
/// this app) flashing during navigation.
#[page]
pub fn page() -> DomNode {
    let resource = create_resource(|| async {
        // Simulate a network fetch that resolves after ~600ms.
        velo::sleep(600).await;
        42u32
    });

    let susp_loading = resource.clone();
    let susp_value = resource.clone();

    view! {
        <div class="page">
            <h1>"Dashboard"</h1>
            <div class="card">
                <Suspense loading={ susp_loading.loading() }
                          fallback={ view!{ <p class="muted">"Loading stats…"</p> } }>
                    <p>"Health score = " { susp_value.value().unwrap_or(0) } " / 100"</p>
                </Suspense>
            </div>
            <Link to={ paths::BROKEN } label="Open the broken subtree" />
        </div>
    }
}