use velo::prelude::*;

/// One of the summary "stat cards" on the dashboard. The value is a reactive
/// expression so it re-renders when its underlying signal/memo changes.
#[component]
pub fn StatsCard(label: &'static str, value: i64) -> DomNode {
    view! {
        <div class="stat-card">
            <h3>{ label.to_string() }</h3>
            <div class="stat-value">{ value }</div>
        </div>
    }
}
