use velo::prelude::*;

use crate::components::*;

/// Home feed at `/`. Renders a **keyed** `SignalVec<Post>` and passes each
/// item to a component as an owned non-`Copy` value (`post={ p.clone() }`).
/// Pushing a post mounts a brand-new card via the keyed DOM reconciler.
#[page]
pub fn page() -> DomNode {
    let feed: SignalVec<Post> = signal_vec(vec![
        Post {
            id: 1,
            title: "The vexing Copy bound".into(),
            author: "rosa".into(),
            body: "Owned props in keyed lists used to be a compile wall. The reconciler now takes Fn(&T), so items reach components by value.".into(),
        },
        Post {
            id: 2,
            title: "Children all the way down".into(),
            author: "dev".into(),
            body: "Nested nodes flow into children: Vec<DomNode>, the same way a segment layout receives its routed page as child.".into(),
        },
    ]);
    let next_id = signal!(3);
    let feed_for_btn = feed.clone();

    view! {
        <div class="page">
            <h1>"Component children & non-Copy keyed lists"</h1>
            <p>
                "Each card below is a "
                <code>"PostCard"</code>
                " rendered inside a keyed "
                <code>"for"</code>
                " over a "
                <code>"SignalVec<Post>"</code>
                " — the item travels as an owned "
                <code>"Post"</code>
                " (`p.clone()`), with no "
                <code>"Copy"</code>
                " bound and no "
                <code>".get()"</code>
                "."
            </p>
            <button class="add-post" on:click={ move |_| {
                let id = next_id.get();
                next_id.set(id + 1);
                feed_for_btn.push(Post {
                    id,
                    title: format!("Reactive post #{id}"),
                    author: "feed".into(),
                    body: "Inserted with SignalVec.push — the keyed reconciler mounts exactly this node.".into(),
                });
            } }>
                "Push a new post"
            </button>
            <div class="feed">
                {
                    for p in feed key = |p: &Post| p.id {
                        <PostCard post={ p.clone() }>
                            <p class="foot-note">"This is " <code>"children"</code> " inside a keyed list item."</p>
                        </PostCard>
                    }
                }
            </div>
        </div>
    }
}