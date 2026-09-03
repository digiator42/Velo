use velo::prelude::*;

/// A single activity row. Takes owned `String` props so the keyed `for` body
/// can pass **eager** clones (rather than `{ ... }` text slots that would
/// capture the borrowed `&Activity` in a `'static` closure).
#[component]
pub fn ActivityItem(user: String, action: String, target: String) -> DomNode {
    view! {
        <div class="activity-item">
            <strong>{ user }</strong>
            { " " }
            { action }
            { " " }
            <span class="target">{ target }</span>
        </div>
    }
}

/// The recent-activity feed. Consumes a `SignalVec<Activity>` so appending
/// activities live-updates the list via the keyed `for` reconciler.
#[component]
pub fn ActivityFeed(activities: velo::SignalVec<crate::api::Activity>) -> DomNode {
    view! {
        <div class="activity-feed">
            {
                for act in activities key = |a: &crate::api::Activity| a.id.clone() {
                    <ActivityItem
                        user={ act.user.clone() }
                        action={ act.action.clone() }
                        target={ act.target.clone() } />
                }
            }
        </div>
    }
}
