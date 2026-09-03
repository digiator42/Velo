use velo::prelude::*;
use std::rc::Rc;

use crate::api::MockApi;
use crate::components::*;

/// Task detail at `/board/:id/task/:taskId`. Loads the project's task list via
/// `create_resource`, resolves the current task by id, and renders it.
///
/// If the task can't be found after loading, it raises a `boundary_fault` so
/// the nearest `error.rs` shows a graceful "task not found" fallback (app!'s
/// generated wrapper provides the boundary).
#[page]
pub fn page() -> DomNode {
    let project_id = FRouter::use_param::<String>("id").unwrap_or_default();
    let task_id = FRouter::use_param::<String>("taskId").unwrap_or_default();

    let tasks_pid = project_id.clone();
    let tasks = create_resource(move || {
        let pid = tasks_pid.clone();
        async move {
            velo::sleep(350).await;
            MockApi::tasks(&pid)
        }
    });

    // Reactive resolution of the current task by id (`memo!`).
    let found = memo!({
        let tasks = tasks.clone();
        let tid = task_id.clone();
        move || tasks.value().as_ref().and_then(|ts| ts.iter().find(|t| t.id == tid).cloned())
    });

    view! {
        <div class="task-detail-page">
            <Head title="Task · Velocity" />
            <Suspense loading={ tasks.loading() }
                      fallback={ view! { <div class="loading">"Loading task…"</div> } }>
                { move || {
                    let close_pid = project_id.clone();
                    match found.get() {
                        Some(t) => view! {
                            <TaskDetail
                                task={ t }
                                project_id={ project_id.clone() }
                                on_close={ Rc::new(move || velo::navigate_to(&paths::board_id(&close_pid))) } />
                        },
                        None => velo::boundary_fault("task not found"),
                    }
                } }
            </Suspense>
        </div>
    }
}
