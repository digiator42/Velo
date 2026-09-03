use velo::prelude::*;

/// A deliberately "heavy" chart subtree that is lazy-loaded via `use_dynamic`
/// on the dashboard. Rendering it simulates a chunk fetch with a short delay,
/// after which the real subtree swaps in.
#[component]
pub fn Chart(stats: crate::api::DashboardStats) -> DomNode {
    view! {
        <div class="chart">
            <h3>"Project Breakdown"</h3>
            <p>"Total tasks: " { stats.total_tasks }</p>
            <p>"Done: " { stats.done_tasks }</p>
            <p>"Completion: " { stats.completion }</p>
        </div>
    }
}
