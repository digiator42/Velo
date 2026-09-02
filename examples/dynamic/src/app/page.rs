use velo::prelude::*;

/// An async-loader helper that "fetches" a whole subtree, exposing the
/// `use_dynamic` swap-in behavior with a visible loading placeholder.
fn heavy_chart() -> DomNode {
    // Loading placeholder shows first; the resolved subtree swaps in ~500ms.
    use_dynamic(
        || async {
            velo::sleep(1200).await;
            view! {
                <div class="chart">
                    <h2>"Heavy chart (lazy-loaded)"</h2>
                    <p>"This subtree was swapped in after the async loader resolved."</p>
                </div>
            }
        },
        view! { <div class="chart placeholder">"Loading heavy chart…"</div> },
    )
}

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="Home · dynamic" />
            <h1>"Async lazy-loading"</h1>
            <p>"The chart below appears after a short async load:"</p>
            { heavy_chart() }
        </div>
    }
}
