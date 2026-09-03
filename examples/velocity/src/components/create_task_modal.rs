use velo::prelude::*;
use std::rc::Rc;

/// A modal form for creating a new task. Exercises `bind:value` (text inputs)
/// and `bind:checked` (priority checkboxes) plus `on:submit`. On submit it
/// calls the provided `on_create` callback with the assembled `Task` and
/// closes itself.
#[component]
pub fn CreateTaskModal(open: bool,
                      project_id: String,
                      column_id: String,
                      on_create: Rc<dyn Fn(crate::api::Task)>,
                      on_close: Rc<dyn Fn()>) -> DomNode {
    let title = signal!(String::new());
    let description = signal!(String::new());
    let assignee = signal!(String::new());
    let pri_high = signal!(true);
    let pri_med = signal!(false);
    let pri_low = signal!(false);

    let on_create_c = on_create.clone();
    let on_close_backdrop = on_close.clone();
    let on_close_submit = on_close.clone();
    let on_close_cancel = on_close.clone();
    let proj = project_id.clone();
    let col = column_id.clone();

    let priority = move || {
        if pri_high.get() { crate::api::Priority::High }
        else if pri_med.get() { crate::api::Priority::Medium }
        else { crate::api::Priority::Low }
    };

    if !open {
        return DomNode::empty();
    }

    view! {
        <div class="overlay-backdrop" on:click={ move |_| on_close_backdrop() }>
            <div class="task-detail" on:click={ move |e: web_sys::Event| e.stop_propagation() }>
                <h2>"New Task"</h2>
                <div class="form-group">
                    <label>"Title"</label>
                    <input type="text" bind:value={ title } placeholder="Task title" />
                </div>
                <div class="form-group">
                    <label>"Description"</label>
                    <textarea bind:value={ description } placeholder="Describe the task..." />
                </div>
                <div class="form-group">
                    <label>"Assignee"</label>
                    <input type="text" bind:value={ assignee } placeholder="Person's name" />
                </div>
                <div class="form-group">
                    <label>"Priority"</label>
                    <label><input type="checkbox" bind:checked={ pri_high } /> "High"</label>
                    <label><input type="checkbox" bind:checked={ pri_med } /> "Medium"</label>
                    <label><input type="checkbox" bind:checked={ pri_low } /> "Low"</label>
                </div>
                <form on:submit={ move |_| {
                    let t = crate::api::MockApi::create_task(
                        &proj, &col, &title.get(), priority(),
                    );
                    on_create_c(t);
                    on_close_submit();
                } }>
                    <button type="submit">"Create"</button>
                    <button type="button" class="secondary" on:click={ move |_| on_close_cancel() }>"Cancel"</button>
                </form>
            </div>
        </div>
    }
}
