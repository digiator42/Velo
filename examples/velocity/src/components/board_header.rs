use velo::prelude::*;
use std::rc::Rc;

/// The board page's top bar: title, a live search box (`bind:value`),
/// quick-add task form (`on:submit` which auto-prevents default), and
/// status-filter toggles (`class:` driven by signals).
#[component]
pub fn BoardHeader(project_name: String,
                   search: RwSignal<String>,
                   show_todo: RwSignal<bool>,
                   show_inprogress: RwSignal<bool>,
                   show_done: RwSignal<bool>,
                   on_add: Rc<dyn Fn(String)>) -> DomNode {
    // Local signal for the quick-add input (distinct from the live search box).
    let new_title = signal!(String::new());
    let on_add_c = on_add.clone();
    let title_for_submit = new_title.clone();

    view! {
        <div class="board-header">
            <h1>{ project_name.clone() }</h1>
            <input type="text" placeholder="Search tasks..." bind:value={ search } class="search-box" />
            <div class="filters">
                <button class:active={ show_todo } on:click={ move |_| show_todo.set(!show_todo.get()) }>"Todo"</button>
                <button class:active={ show_inprogress } on:click={ move |_| show_inprogress.set(!show_inprogress.get()) }>"In Progress"</button>
                <button class:active={ show_done } on:click={ move |_| show_done.set(!show_done.get()) }>"Done"</button>
            </div>
            <div class="quick-add">
                <form on:submit={ move |_| {
                    let t = title_for_submit.get().trim().to_string();
                    if !t.is_empty() {
                        on_add_c(t);
                        title_for_submit.set(String::new());
                    }
                } }>
                    <input type="text" placeholder="Type a task and press Enter..." bind:value={ new_title } />
                    <button type="submit">"Add"</button>
                </form>
            </div>
        </div>
    }
}
