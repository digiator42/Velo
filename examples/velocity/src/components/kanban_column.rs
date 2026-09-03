use velo::prelude::*;
use std::rc::Rc;
use crate::components::*;

/// A single Kanban column: header (title + reactive count) + a keyed `for`
/// over this column's `SignalVec<Task>`. Reorders/removes only touch the
/// affected card node (reconciled by id).
#[component]
pub fn KanbanColumn(
    column: crate::api::Column,
    tasks: velo::SignalVec<crate::api::Task>,
    on_task_open: Rc<dyn Fn(String)>,
) -> DomNode {
    let col_id = column.id.clone();
    // Reactive count of tasks in this column (`memo!`). Clone the SignalVec so
    // the memo's `move` closure doesn't consume `tasks` before the `for` below.
    let tasks_for_count = tasks.clone();
    let count = memo!(move || {
        tasks_for_count.get().iter().filter(|t| t.column_id == col_id).count()
    });
    view! {
        <div class="kanban-column">
            <div class="column-header">
                { column.title.clone() }
                <span class="column-count">{ count }</span>
            </div>
            { for t in tasks key = |t: &crate::api::Task| t.id.clone() {
                <TaskCard task={ t.clone() } on_open={ on_task_open.clone() } />
            } }
        </div>
    }
}
