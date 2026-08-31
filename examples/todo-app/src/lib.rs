use velo::prelude::*;
use velo_dom::mount_to_id;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::components::{AppState, Task, TaskRow};

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
                <input
                    type="text"
                    placeholder="What needs doing?"
                    bind:value={ input }
                />
                <button on:click={ move |_| {
                    let text = input.get().trim().to_string();
                    if text.is_empty() { return; }
                    let next = s_add.tasks.get().iter().map(|t| t.id).max().unwrap_or(0) + 1;
                    s_add.tasks.push(Task { id: next, title: text, done: false });
                    input.set(String::new());
                }}>"Add"</button>
            </div>

            <div class="stats">
                <div class="stat-card">
                    <div class="label">"Total"</div>
                    <div class="value">{ total.get() }</div>
                </div>
                <div class="stat-card">
                    <div class="label">"Open"</div>
                    <div class="value">{ open.get() }</div>
                </div>
                <div class="stat-card">
                    <div class="label">"Done"</div>
                    <div class="value">{ done.get() }</div>
                </div>
            </div>

            <div>
                {
                    for t in st_for.tasks key = |t: &Task| t.id {
                        <TaskRow task={ t.clone() } />
                    }
                }
            </div>

            {
                move || {
                    if state.tasks.get().is_empty() {
                        velo_dom::DomNode::text("No tasks yet — add one above.")
                    } else {
                        velo_dom::DomNode::text("")
                    }
                }
            }
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
                            velo_dom::DomNode::text("All caught up!")
                        } else {
                            let frag = velo_dom::DomNode::fragment();
                            for t in &titles {
                                let li = velo_dom::DomNode::element("li");
                                li.append(&velo_dom::DomNode::text(t));
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

    mount_to_id("app", app_shell);
}
