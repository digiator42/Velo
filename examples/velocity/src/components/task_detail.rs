use velo::prelude::*;
use std::rc::Rc;
use crate::components::*;

/// Modal overlay showing a single task's detail (title, description,
/// assignee, created-at, priority/status badges). Closed via the backdrop or
/// the close button, which navigates back to the board
/// (`route_path!`-validated programmatic navigation).
#[component]
pub fn TaskDetail(task: crate::api::Task, project_id: String, on_close: Rc<dyn Fn()>) -> DomNode {
    let on_close_c = on_close.clone();
    let on_close_c2 = on_close.clone();
    let pid = project_id.clone();

    view! {
        <div class="overlay-backdrop" on:click={ move |_| on_close_c() }>
            <div class="task-detail" on:click={ move |_| {} }>
                <button class="close" on:click={ move |_| on_close_c2()}>"x"</button>
                <h2>{ task.title.clone() }</h2>
                <p class="label">"Description"</p>
                <p class="value">{ task.description.clone() }</p>
                <p class="label">"Assignee"</p>
                <p class="value">{ task.assignee.clone() }</p>
                <p class="label">"Created"</p>
                <p class="value">{ task.created_at.clone() }</p>
                <div class="task-meta">
                    <PriorityBadge priority={ task.priority } />
                    <StatusBadge status={ task.status } />
                </div>
                <button on:click={ move |_| navigate_to(&format!("/board/{}", pid)) }>"Back to board"</button>
            </div>
        </div>
    }
}
