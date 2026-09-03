use velo::prelude::*;
use std::rc::Rc;
use crate::components::*;

/// The Kanban board view: renders columns side-by-side via a keyed `for` over a
/// `SignalVec<Column>`. Each column receives the shared grouped memo
/// (`Vec<(Column, Vec<Task>)>`) plus its own id and filters its tasks
/// reactively, so search/status/toasts flow through one reactive source.
#[component]
pub fn KanbanBoard(
    columns: velo::SignalVec<crate::api::Column>,
    grouped: velo::Memo<Vec<(crate::api::Column, Vec<crate::api::Task>)>>,
    on_task_open: Rc<dyn Fn(String)>,
) -> DomNode {
    let on_open = on_task_open.clone();
    view! {
        <div class="board-view">
            { for col in columns key = |c: &crate::api::Column| c.id.clone() {
                <KanbanColumn
                    column={ col.clone() }
                    grouped={ grouped.clone() }
                    on_task_open={ on_open.clone() } />
            } }
        </div>
    }
}
