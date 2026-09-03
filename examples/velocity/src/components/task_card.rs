use velo::prelude::*;
use crate::components::*;
use std::rc::Rc;

/// A single task card rendered inside a Kanban column. Clicking opens the
/// task detail via the `on_open` callback (programmatic navigation).
/// The overdue flag drives a `class:overdue` toggle; priority/status use
/// `class_names!`-backed badge components.
#[component]
pub fn TaskCard(task: crate::api::Task, on_open: Rc<dyn Fn(String)>) -> DomNode {
    let task_id = task.id.clone();
    let is_overdue = crate::api::MockApi::is_overdue(&task);
    let open = on_open.clone();
    view! {
        <div class="task-card" class:overdue={ is_overdue }
             on:click={ move |_| open(task_id.clone()) }>
            <div class="task-card-header">
                <PriorityBadge priority={ task.priority.clone() } />
                <StatusBadge status={ task.status.clone() } />
            </div>
            <div class="task-title">{ task.title.clone() }</div>
            <div class="task-assignee">{ "  " task.assignee.clone() }</div>
        </div>
    }
}
