use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::components::{AppState, Task, TaskRow, TaskRowProps};

mod components;

fn home_page() -> DomNode {
    view! {
        <div class="page">
            <h1>"Task Manager"</h1>
            <p>"A small Velo application exercising routing, reactive lists, derived state, context, and the #[component] macro."</p>
            <p>"Use the nav above to switch between Tasks and Stats."</p>
        </div>
    }
}

fn tasks_page() -> DomNode {
    let state = use_context::<AppState>().expect("AppState must be provided at root");
    // `bind:value` treats this as a controlled input: typing writes the signal,
    // and resetting the signal below (after "Add") writes back to the live DOM
    // property, so the field clears even though we never touched the element.
    let input = signal(String::new());

    let s_add = state.clone();

    // Derived, reactive stats from the list (recompute only on change).
    let total = memo({ let t = state.tasks.clone(); move || t.get().len() as u32 });
    let open = memo({
        let t = state.tasks.clone();
        move || t.get().iter().filter(|x| !x.done).count() as u32
    });
    let done = memo({
        let t = state.tasks.clone();
        move || t.get().iter().filter(|x| x.done).count() as u32
    });

    let st_for = state.clone();

    view! {
        <div class="page">
            <h1>"Tasks"</h1>

            <div class="adder">
                <form on:submit={ () => {
                    let text = input.get().trim().to_string();
                    if text.is_empty() { return; }
                    let next = s_add.tasks.get().iter().map(|t| t.id).max().unwrap_or(0) + 1;
                    s_add.tasks.push(Task { id: next, title: text, done: false });
                    // Controlled reset: bind:value pushes this back to the input.
                    input.set(String::new());
                } }>
                    <input
                        type="text"
                        placeholder="What needs doing?"
                        bind:value={ input }
                    />
                    <button type="submit">"Add"</button>
                </form>
            </div>

            <div class="stats">
                <div class="stat-card">
                    <div class="label">"Total"</div>
                    <div class="value">{ total }</div>
                </div>
                <div class="stat-card">
                    <div class="label">"Open"</div>
                    <div class="value">{ open }</div>
                </div>
                <div class="stat-card">
                    <div class="label">"Done"</div>
                    <div class="value">{ done }</div>
                </div>
            </div>

            <div>
                {
                    for t in st_for.tasks key = |t: &Task| t.id {
                        <TaskRow task={ t.clone() } />
                    }
                }
            </div>

            <Show when={ move || state.tasks.get().is_empty() } fallback={ DomNode::text("") }>
                <div class="empty-state">
                    <p>"No tasks yet — add one above."</p>
                </div>
            </Show>
        </div>
    }
}

fn stats_page() -> DomNode {
    let state = use_context::<AppState>().expect("AppState must be provided at root");

    let open_titles = memo({
        let t = state.tasks.clone();
        move || t.get().iter().filter(|x| !x.done).map(|x| x.title.clone()).collect::<Vec<_>>()
    });

    view! {
        <div class="page">
            <h1>"Open tasks"</h1>
            <ul>
                {
                    move || {
                        let titles = open_titles.get();
                        if titles.is_empty() {
                            DomNode::text("All caught up!")
                        } else {
                            let frag = DomNode::fragment();
                            for t in &titles {
                                let li = DomNode::element("li");
                                li.append(&DomNode::text(t));
                                frag.append(&li);
                            }
                            frag
                        }
                    }
                }
            </ul>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    run_app();
}

pub fn run_app() {
    // Share the task list across all pages via context.
    provide_context(AppState {
        tasks: signal_vec(vec![
            Task { id: 1, title: "Learn Velo's reactivity".into(), done: false },
            Task { id: 2, title: "Build a SPA".into(), done: true },
        ]),
    });

    let routes = vec![
        Route { path: "/", component: home_page },
        Route { path: "/tasks", component: tasks_page },
        Route { path: "/stats", component: stats_page },
        Route { path: "/**", component: home_page },
    ];

    let app_shell = view! {
        <div id="app-container">
            <nav>
                <span class="brand">"Velo"</span>
                <Link to="/" label="Home" />
                <Link to="/tasks" label="Tasks" />
                <Link to="/stats" label="Stats" />
            </nav>
            <Router routes={ routes } />
        </div>
    };

    mount(app_shell);
}
