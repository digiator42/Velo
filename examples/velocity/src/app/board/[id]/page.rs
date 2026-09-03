use velo::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

use crate::api::{Column, MockApi, Priority, Status, Task};
use crate::components::*;

/// The Kanban board at `/board/:id`. Loads the project, its columns, and its
/// tasks via `create_resource` + `<Suspense>`, then hands reactive
/// `SignalVec`s to `<KanbanBoard>` (keyed `for` reconciliation).
///
/// Exercises `signal!` + `memo!`, `signal_vec` + keyed `for`, `effect!`,
/// `class:` toggles, `class_names!`, `<Link prefetch>`, `route_path!` (via
/// `paths::`), async `() => {}` arrow handlers, and `use_context` (theme).
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

    // ---- Reactive column list (`memo!`) filtered by search ----
    let columns_mv = memo!({
        let cols = cols_raw.clone();
        let s = search.clone();
        move || {
            let q = s.get().to_lowercase();
            cols.value()
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter(|c| c.title.to_lowercase().contains(&q))
                .collect::<Vec<Column>>()
        }
    });
    let columns_sv = velo::signal_vec(columns_mv.get());
    let sync_cols = columns_sv.clone();
    let sync_cols_src = columns_mv.clone();
    effect!(move || {
        sync_cols.with_mut(|v| *v = sync_cols_src.get());
    });

    // ---- Per-column task lists (`memo!` filtered by search + status) ----
    let tasks_filtered_mv = memo!({
        let tasks = tasks_raw.clone();
        let (st, si, sd) = (show_todo, show_inprogress, show_done);
        let s = search.clone();
        move || {
            let q = s.get().to_lowercase();
            tasks.value().clone().unwrap_or_default().into_iter()
                .filter(|t| {
                    let matches_search = t.title.to_lowercase().contains(&q);
                    let matches_status = match t.status {
                        Status::Todo => st.get(),
                        Status::InProgress => si.get(),
                        Status::Done => sd.get(),
                    };
                    matches_search && matches_status
                })
                .collect::<Vec<Task>>()
        }
    });

    // One SignalVec per column, synced with the filtered task memo via an effect.
    let tasks_by_col: Rc<std::cell::RefCell<HashMap<String, velo::SignalVec<Task>>>> =
        Rc::new(std::cell::RefCell::new(HashMap::new()));
    // Seed column buckets from the initial filtered snapshot.
    {
        let cols = columns_mv.get();
        let tasks_snapshot = tasks_filtered_mv.get();
        let mut map = tasks_by_col.borrow_mut();
        for c in &cols {
            let in_col: Vec<Task> = tasks_snapshot
                .iter()
                .filter(|t| t.column_id == c.id)
                .cloned()
                .collect();
            map.insert(c.id.clone(), velo::signal_vec(in_col));
        }
    }
    let tasks_for_col = {
        let map = Rc::clone(&tasks_by_col);
        Rc::new(move |col_id: &str| -> velo::SignalVec<Task> {
            map.borrow().get(col_id).cloned().unwrap_or_else(|| velo::signal_vec(Vec::new()))
        }) as Rc<dyn Fn(&str) -> velo::SignalVec<Task>>
    };

    // Sync each column's SignalVec with the filtered memo on changes.
    let sync_tasks = tasks_by_col.clone();
    let sync_tasks_src = tasks_filtered_mv.clone();
    effect!(move || {
        let snapshot = sync_tasks_src.get();
        let map = sync_tasks.borrow();
        for (id, sv) in map.iter() {
            let in_col: Vec<Task> = snapshot.iter().filter(|t| t.column_id == *id).cloned().collect();
            sv.with_mut(|v| *v = in_col);
        }
    });

    // Create-task handler: append to the mock store AND the column's SignalVec.
    let on_add: Rc<dyn Fn(String)> = {
        let project_id = project_id.clone();
        let cols = cols_raw.clone();
        let tbc_lookup = tasks_for_col.clone();
        Rc::new(move |title: String| {
            let col_id = cols
                .value()
                .as_ref()
                .and_then(|cs| cs.first().map(|c| c.id.clone()))
                .unwrap_or_default();
            let t = MockApi::create_task(&project_id, &col_id, &title, Priority::Medium);
            tbc_lookup(&col_id).push(t);
        })
    };

    // Create-task handler for the modal: the modal builds the `Task` itself
    // (via MockApi::create_task, which persists it), so we just push the
    // returned task into its column's SignalVec. Keeping it separate from
    // `on_add` (title-only, for the quick-add form) matches the two shapes.
    let on_create_task: Rc<dyn Fn(Task)> = {
        let tbc_lookup = tasks_for_col.clone();
        Rc::new(move |t: Task| {
            tbc_lookup(&t.column_id).push(t);
        })
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

            <Suspense loading={ cols_raw.loading() || tasks_raw.loading() }
                      fallback={ view! { <div class="loading">"Loading board…"</div> } }>
                <KanbanBoard
                    columns={ columns_sv }
                    tasks_for_column={ tasks_for_col.clone() }
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
