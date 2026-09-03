use velo::prelude::*;

/// A priority pill rendered with `class_names!` so the CSS class is derived
/// from the task's priority variant (High/Medium/Low).
#[component]
pub fn PriorityBadge(priority: crate::api::Priority) -> DomNode {
    let label = match priority {
        crate::api::Priority::High => "High",
        crate::api::Priority::Medium => "Medium",
        crate::api::Priority::Low => "Low",
    };
    let class = match priority {
        crate::api::Priority::High => "badge--high",
        crate::api::Priority::Medium => "badge--medium",
        crate::api::Priority::Low => "badge--low",
    };
    view! {
        <span class={ class_names!("badge", class) }>{ label.to_string() }</span>
    }
}
