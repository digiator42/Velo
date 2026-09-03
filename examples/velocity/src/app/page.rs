use velo::prelude::*;

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
        velo::sleep(400).await;
        MockApi::projects()
    });
    let stats = create_resource(|| async {
        velo::sleep(500).await;
        MockApi::dashboard_stats()
    });
    let activities = create_resource(|| async {
        velo::sleep(600).await;
        MockApi::activities()
    });

    // `loading()` is consumed inside `<Suspense>`'s reactive predicate closure,
    // so hand each resource a dedicated clone for that role (Resource is cheap
    // to clone / Rc-backed). Same for `value()` used in the stats match blocks.
    let projects_loading = projects.clone();
    let stats_loading_overview = stats.clone();
    let stats_loading_chart = stats.clone();
    let activities_loading = activities.clone();
    let stats_value_overview = stats.clone();
    let stats_value_chart = stats.clone();

    // ---- Live search: signal! + memo! (filtered count recomputes on type) ----
    let search = signal!(String::new());
    let proj_for_filter = projects.clone();
    let filtered_count = memo!(move || {
        let p = proj_for_filter.value();
        if let Some(projects) = p {
            let q = search.get().to_lowercase();
            let mut count = 0usize;
            for proj in &projects {
                let tasks = MockApi::tasks(&proj.id);
                for t in &tasks {
                    if t.title.to_lowercase().contains(&q) {
                        count += 1;
                    }
                }
            }
            count
        } else {
            0
        }
    });

    view! {
        <div class="dashboard">
            <Head title="Dashboard · Velocity" />
            <h1>"Dashboard"</h1>
            <p class="subtitle">"Overview of all your projects and recent activity."</p>

            <Suspense loading={ projects_loading.loading() }
                      fallback={ view! { <div class="loading">"Loading projects…"</div> } }>
                <ProjectsSection projects={ projects.clone() } />
            </Suspense>

            <div class="search-bar">
                <input type="text" placeholder="Search tasks..." bind:value={ search } />
                <span class="hint">"Matching tasks: " { filtered_count }</span>
            </div>

            <Suspense loading={ stats_loading_overview.loading() }
                      fallback={ view! { <div class="loading">"Loading stats…"</div> } }>
                { move || match stats_value_overview.value() {
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

            <Suspense loading={ stats_loading_chart.loading() }
                      fallback={ view! { <div class="loading">"Loading chart…"</div> } }>
                { move || match stats_value_chart.value() {
                    Some(s) => {
                        let chart_node = use_dynamic(
                            move || async move {
                                velo::sleep(900).await;
                                view! { <Chart stats={ s.clone() } /> }
                            },
                            view! { <div class="chart placeholder">"Loading chart…"</div> },
                        );
                        view! { <section class="chart-section">{ chart_node }</section> }
                    }
                    None => view! { <div class="loading">"Waiting for stats…"</div> },
                } }
            </Suspense>

            <Suspense loading={ activities_loading.loading() }
                      fallback={ view! { <div class="loading">"Loading activity…"</div> } }>
                <ActivitySection activities={ activities.clone() } />
            </Suspense>

            <KeyboardShortcuts />
        </div>
    }
}

/// Renders the projects list with prefetch `<Link>`s (board list → board detail).
/// Exercises `<Link prefetch>` promise sharing across navigations.
#[component]
fn ProjectsSection(projects: velo::Resource<Vec<crate::api::Project>>) -> DomNode {
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
fn ActivitySection(activities: velo::Resource<Vec<crate::api::Activity>>) -> DomNode {
    let acts = activities.clone();
    view! {
        <div>
            <h2>"Recent Activity"</h2>
            { move || {
                let a = acts.value();
                match a {
                    Some(list) if !list.is_empty() => {
                        let list_sv = velo::signal_vec(list);
                        view! { <ActivityFeed activities={ list_sv } /> }
                    }
                    _ => view! { <p class="subtitle">"No activity yet."</p> },
                }
            } }
        </div>
    }
}
