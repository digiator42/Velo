use velo::prelude::*;

/// A post item — deliberately **non-`Copy`**: it owns `String` fields. Passing
/// it into components by value (`<PostCard post={ p.clone() } />`) inside a
/// keyed `for` used to be a compile wall when the per-item render closure was
/// `Fn(T) -> DomNode` + `Copy`; today the keyed reconciler takes
/// `Fn(&T) -> DomNode` and `#[component]` props are plain owned values.
#[derive(Clone)]
pub struct Post {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub body: String,
}

/// A card taking an owned non-`Copy` `Post` plus named `children`. Renders
/// inside the keyed feed on "/" — proving the 5.P2 fix end to end.
#[allow(non_snake_case)]
#[component]
pub fn PostCard(post: Post, children: Vec<DomNode>) -> DomNode {
    view! {
        <article class="post-card">
            <h2>{ post.title.clone() }</h2>
            <p class="meta">"by " { post.author.clone() }</p>
            <p class="body">{ post.body.clone() }</p>
            <footer class="card-footer">{ children }</footer>
        </article>
    }
}

/// A generic wrapper: `children` flow in as nested nodes, the same way a
/// segment `layout(child: DomNode)` receives its routed page.
#[allow(non_snake_case)]
#[component]
pub fn Panel(title: String, children: Vec<DomNode>) -> DomNode {
    view! {
        <section class="panel">
            <h3 class="panel-title">{ title }</h3>
            <div class="panel-body">{ children }</div>
        </section>
    }
}