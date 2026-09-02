use velo::prelude::*;

/// Nested segment layout for `/posts` — receives its page as `child` and wraps
/// it, proving layouts compose like components: root shell → segment shell →
/// leaf. Its own counter demonstrates segment-shell persistence on M4.
#[layout]
pub fn layout(child: DomNode) -> DomNode {
    let opens = signal!(0);
    view! {
        <div class="posts-wrap">
            <div class="posts-toolbar">
                <span class="pill">"posts segment layout"</span>
                <button class="seg-count" on:click={ move |_| opens.set(opens.get() + 1) }>
                    "segment count: " { opens }
                </button>
            </div>
            { child }
        </div>
    }
}