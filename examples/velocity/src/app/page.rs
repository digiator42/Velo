use veloasm::prelude::*;

use crate::api::MockApi;
use crate::components::*;

/// The dashboard at `/`. Loads projects + aggregate stats + activity via
/// `create_resource` with `<Suspense>` fallbacks, renders summary stats cards,
/// a live search box (`signal!` + `memo!` filtered count), and a lazy-loaded
/// chart via `use_dynamic` (with a visible loading placeholder).
#[page]
pub fn page() -> DomNode {
    // ---- Async resources (mock latency so Suspense loading states show) ----
    let projects = create_resource(|| async {
        veloasm::sleep(400).await;
        MockApi::projects()
    });
    let stats = create_resource(|| async {
        veloasm::sleep(500).await;
        MockApi::dashboard_stats()
    });
    let activities = create_resource(|| async {
        veloasm::sleep(600).await;
        MockApi::activities()
    });

    // `Resource` and `Memo` are `Copy`: the originals move into every
    // reactive closure below with no clones.
    // ---- Live search: signal! + memo! (filtered count + results recomputes on type) ----
    let search = signal!(String::new());
    let filtered_tasks = memo!(move || {
        let p = projects.value();
        if let Some(projects) = p {
            let q = search.get().to_lowercase();
            if q.is_empty() {
                return Vec::new();
            }
            let mut results = Vec::new();
            for proj in &projects {
                let tasks = MockApi::tasks(&proj.id);
                for t in &tasks {
                    if t.title.to_lowercase().contains(&q) {
                        results.push((proj.name.clone(), t.clone()));
                    }
                }
            }
            results
        } else {
            Vec::new()
        }
    });
    let filtered_count = memo!(move || filtered_tasks.get().len());

    view! {
        <div class="dashboard">
            <Head title="Dashboard · Velocity" />
            <h1>"Dashboard"</h1>
            <p class="subtitle">"Overview of all your projects and recent activity."</p>

            <Suspense loading={ move || projects.loading() }
                      fallback={ view! { <div class="loading">"Loading projects…"</div> } }>
                <ProjectsSection projects={ projects } />
            </Suspense>

            <div class="search-bar">
                <input type="text" placeholder="Search tasks..." bind:value={ search } />
                <span class="hint">"Matching tasks: " { filtered_count }</span>
            </div>

            { move || {
                let tasks = filtered_tasks.get();
                if tasks.is_empty() {
                    return view! { <div></div> };
                }
                let items = tasks.into_iter().enumerate().map(|(i, (proj_name, task))| {
                    let title = task.title.clone();
                    let proj = proj_name.clone();
                    view! {
                        <div class="activity-item" key={ i.to_string() }>
                            <span class="target">{ proj }</span>
                            " — " { title }
                        </div>
                    }
                }).collect::<Vec<_>>();
                view! { <div class="search-results activity-feed">{ items }</div> }
            } }

            <Suspense loading={ move || stats.loading() }
                      fallback={ view! { <div class="loading">"Loading stats…"</div> } }>
                { move || match stats.value() {
                    Some(s) => view! {
                        <div class="stats-grid">
                            <StatsCard label="Projects" value={ s.projects as i64 } />
                            <StatsCard label="Total Tasks" value={ s.total_tasks as i64 } />
                            <StatsCard label="Done" value={ s.done_tasks as i64 } />
                            <StatsCard label="In Progress" value={ s.in_progress as i64 } />
                            <StatsCard label="Overdue" value={ s.overdue as i64 } />
                        </div>
                    },
                    None => view! { <div class="loading">"No stats available."</div> },
                } }
            </Suspense>

            <Suspense loading={ move || stats.loading() }
                      fallback={ view! { <div class="loading">"Loading chart…"</div> } }>
                { move || match stats.value() {
                    Some(s) => {
                        let chart_node = use_dynamic(
                            move || async move {
                                veloasm::sleep(900).await;
                                view! { <Chart stats={ s } /> }
                            },
                            view! { <div class="chart placeholder">"Loading chart…"</div> },
                        );
                        view! { <section class="chart-section">{ chart_node }</section> }
                    }
                    None => view! { <div class="loading">"Waiting for stats…"</div> },
                } }
            </Suspense>

            <Suspense loading={ move || activities.loading() }
                      fallback={ view! { <div class="loading">"Loading activity…"</div> } }>
                <ActivitySection activities={ activities } />
            </Suspense>

            <KeyboardShortcuts />
        </div>
    }
}

/// Renders the projects list with prefetch `<Link>`s (board list → board detail).
/// Exercises `<Link prefetch>` promise sharing across navigations.
#[component]
fn ProjectsSection(projects: veloasm::Resource<Vec<crate::api::Project>>) -> DomNode {
    view! {
        <section class="projects-section">
            <h2>"Your Projects"</h2>
            { move || match projects.value() {
                Some(list) => view! {
                    <div class="board-list">
                        { move || list.iter().map(|p| {
                            let pid = p.id.clone();
                            let pname = p.name.clone();
                            view! { <Link to={ paths::board_id(&pid) } prefetch>{ pname }</Link> }
                        }).collect::<Vec<_>>()}
                    </div>
                },
                None => view! { <p class="subtitle">"Loading projects…"</p> },
            } }
        </section>
    }
}

/// Renders the activity feed once the activities resource resolves.
#[component]
fn ActivitySection(activities: veloasm::Resource<Vec<crate::api::Activity>>) -> DomNode {
    view! {
        <div>
            <h2>"Recent Activity"</h2>
            { move || {
                let a = activities.value();
                match a {
                    Some(list) if !list.is_empty() => {
                        let list_sv = veloasm::signal_vec(list);
                        view! { <ActivityFeed activities={ list_sv } /> }
                    }
                    _ => view! { <p class="subtitle">"No activity yet."</p> },
                }
            } }
        </div>
    }
}

