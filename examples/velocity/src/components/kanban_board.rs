use velo::prelude::*;
use std::rc::Rc;
use crate::components::*;

/// The Kanban board view: renders columns side-by-side via a keyed `for` over a
/// `SignalVec<Column>`. Each column receives its own per-column `SignalVec`
/// (looked up via `tasks_for_column`) so reordering/removing tasks is reconciled
/// by id with surgical DOM updates.
#[component]
pub fn KanbanBoard(
    columns: velo::SignalVec<crate::api::Column>,
    tasks_for_column: Rc<dyn Fn(&str) -> velo::SignalVec<crate::api::Task>>,
    on_task_open: Rc<dyn Fn(String)>,
) -> DomNode {
    let on_open = on_task_open.clone();
    view! {
        <div class="board-view">
            { for col in columns key = |c: &crate::api::Column| c.id.clone() {
                <KanbanColumn
                    column={ col.clone() }
                    tasks={ tasks_for_column(&col.id) }
                    on_task_open={ on_open.clone() } />
            } }
        </div>
    }
}
