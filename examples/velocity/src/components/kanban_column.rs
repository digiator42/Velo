use veloasm::prelude::*;
use std::rc::Rc;
use crate::components::*;

/// A single Kanban column. It receives the grouped memo
/// (`Vec<(Column, Vec<Task>)>`) and its own id, filters the matching task list
/// reactively (`memo!` for the count + a `{ move || }` list), and renders a
/// `TaskCard` per task.
#[component]
pub fn KanbanColumn(
    column: crate::api::Column,
    grouped: veloasm::Memo<Vec<(crate::api::Column, Vec<crate::api::Task>)>>,
    on_task_open: Rc<dyn Fn(String)>,
) -> DomNode {
    let col_id = column.id.clone();
    let col_id_for_memo = col_id.clone();
    // Reactive task list for this column, derived from the shared grouped memo.
    let tasks_memo = memo!(move || {
        grouped
            .get()
            .into_iter()
            .find(|(c, _)| c.id == col_id_for_memo)
            .map(|(_, ts)| ts)
            .unwrap_or_default()
    });
    let count = memo!(move || tasks_memo.get().len());
    let on_open = on_task_open.clone();
    view! {
        <div class="kanban-column">
            <div class="column-header">
                { column.title.clone() }
                <span class="column-count">{ count }</span>
            </div>
            { move || {
                let tasks = tasks_memo.get();
                tasks.into_iter().map(|t| {
                    view! { <TaskCard task={ t } on_open={ on_open.clone() } /> }
                }).collect::<Vec<_>>()
            } }
        </div>
    }
}

