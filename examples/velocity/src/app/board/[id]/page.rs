use velo::prelude::*;
use std::rc::Rc;

use crate::api::{Column, MockApi, Priority, Status, Task};
use crate::components::*;

/// The Kanban board at `/board/:id`. Loads the project, its columns, and its
/// tasks via `create_resource` + `<Suspense>`, then derives reactive columns
/// and per-column task lists from a single grouped `memo!`.
///
/// Exercises `signal!` + `memo!`, `effect!`, `class:` toggles, `bind:value`,
/// `<Link prefetch>`, `route_path!` (via `paths::`), async arrow handlers, and
/// `use_context` (theme).
#[page]
pub fn page() -> DomNode {
    let project_id = FRouter::use_param::<String>("id").unwrap_or_default();

    let pid1 = project_id.clone();
    let project = create_resource(move || {
        let pid = pid1.clone();
        async move {
            velo::sleep(300).await;
            MockApi::project(&pid)
        }
    });
    let pid2 = project_id.clone();
    let cols_raw = create_resource(move || {
        let pid = pid2.clone();
        async move {
            velo::sleep(400).await;
            MockApi::columns(&pid)
        }
    });
    let pid3 = project_id.clone();
    let tasks_raw = create_resource(move || {
        let pid = pid3.clone();
        async move {
            velo::sleep(500).await;
            MockApi::tasks(&pid)
        }
    });

    // ---- Search + status filter state (`signal!`) ----
    let search = signal!(String::new());
    let show_todo = signal!(true);
    let show_inprogress = signal!(true);
    let show_done = signal!(true);
    let modal_open = signal!(false);

    // ---- Tasks held in a writable signal so creation re-renders the board ----
    let tasks = signal!(Vec::new());
    let tasks_set = tasks;
    let tasks_raw_for_effect = tasks_raw.clone();
    effect!(move || {
        if let Some(v) = tasks_raw_for_effect.value() {
            tasks_set.set(v);
        }
    });

    // ---- Single grouped memo: (columns filtered by search) × (their tasks,
    //      filtered by search + status). Recomputes on any input signal change.
    let cols_for_grouped = cols_raw.clone();
    let tasks_for_grouped = tasks;
    let grouped = memo!(move || {
        let q = search.get().to_lowercase();
        let cols = cols_for_grouped.value().clone().unwrap_or_default();
        let all_tasks = tasks_for_grouped.get();
        let result = cols.into_iter()
            .filter(|c| c.title.to_lowercase().contains(&q))
            .map(|c| {
                let mine = all_tasks
                    .iter()
                    .filter(|t| {
                        t.column_id == c.id
                            && t.title.to_lowercase().contains(&q)
                            && match t.status {
                                Status::Todo => show_todo.get(),
                                Status::InProgress => show_inprogress.get(),
                                Status::Done => show_done.get(),
                            }
                    })
                    .cloned()
                    .collect::<Vec<Task>>();
                (c, mine)
            })
            .collect::<Vec<(Column, Vec<Task>)>>();
        result
    });

    // Outer keyed `for` reconciles columns; driven by a SignalVec of columns.
    // Starts empty and is populated reactively by `sync_cols` below — we must
    // NOT seed it from `grouped.get()` here, because that eager read would run
    // inside the Router's render effect and leak a subscription into it,
    // causing the Router to re-render (and rebuild the whole page) whenever the
    // memo changes — an infinite loop.
    let columns = velo::signal_vec(Vec::new());
    let sync_cols = columns.clone();
    let grouped_c = grouped.clone();
    effect!(move || {
        sync_cols.with_mut(|v| *v = columns_from(&grouped_c.get()));
    });

    // ---- Create-task handlers: push to the live task signal. ----
    let on_add: Rc<dyn Fn(String)> = {
        let project_id = project_id.clone();
        let cols = cols_raw.clone();
        let tasks = tasks.clone();
        let first_col = move || {
            cols.value()
                .as_ref()
                .and_then(|cs| cs.first().map(|c| c.id.clone()))
                .unwrap_or_default()
        };
        Rc::new(move |title: String| {
            let col_id = first_col();
            let t = MockApi::create_task(&project_id, &col_id, &title, Priority::Medium);
            tasks.update(|v| v.push(t));
        })
    };

    let on_create_task: Rc<dyn Fn(Task)> = {
        let tasks = tasks.clone();
        Rc::new(move |t: Task| tasks.update(|v| v.push(t)))
    };

    let on_task_open = {
        let pid = project_id.clone();
        Rc::new(move |tid: String| {
            velo::navigate_to(&paths::board_task_id_taskid(&pid, &tid));
        }) as Rc<dyn Fn(String)>
    };

    let modal_open_c = modal_open.clone();
    let cols_raw_modal = cols_raw.clone();

    view! {
        <div class="board-page">
            <Head title={ format!("Board: {} · Velocity", project_id) } />
            { move || {
                let name = project.value().as_ref()
                    .and_then(|p| p.as_ref().map(|p| p.name.clone()))
                    .unwrap_or_default();
                view! {
                    <BoardHeader
                        project_name={ name }
                        search={ search.clone() }
                        show_todo={ show_todo }
                        show_inprogress={ show_inprogress }
                        show_done={ show_done }
                        on_add={ on_add.clone() } />
                }
            } }

            <Suspense loading={ move || cols_raw.loading() || tasks_raw.loading() }
                      fallback={ view! { <div class="loading">"Loading board…"</div> } }>
                <KanbanBoard
                    columns={ columns }
                    grouped={ grouped.clone() }
                    on_task_open={ on_task_open } />
            </Suspense>

            { move || if modal_open.get() {
                view! {
                    <CreateTaskModal
                        open={ modal_open.get() }
                        project_id={ project_id.clone() }
                        column_id={ cols_raw_modal.value().as_ref().and_then(|c| c.first().map(|x| x.id.clone())).unwrap_or_default() }
                        on_create={ on_create_task.clone() }
                        on_close={ Rc::new(move || modal_open.set(false)) } />
                }
            } else {
                DomNode::empty()
            } }

            <div class="board-actions">
                <Link to={ paths::INDEX } label="Back to dashboard" prefetch />
                <button on:click={ move |_| modal_open_c.set(true) }>"New Task"</button>
            </div>
        </div>
    }
}

/// Extract just the columns from the grouped memo (for the outer keyed `for`).
fn columns_from(groups: &[(Column, Vec<Task>)]) -> Vec<Column> {
    groups.iter().map(|(c, _)| c.clone()).collect()
}
