use velo::prelude::*;

/// A status badge whose class is computed with `class_names!` from the task
/// status (Todo / InProgress / Done).
#[component]
pub fn StatusBadge(status: crate::api::Status) -> DomNode {
    let label = match status {
        crate::api::Status::Todo => "Todo",
        crate::api::Status::InProgress => "In Progress",
        crate::api::Status::Done => "Done",
    };
    let class = match status {
        crate::api::Status::Todo => "badge--todo",
        crate::api::Status::InProgress => "badge--in-progress",
        crate::api::Status::Done => "badge--done",
    };
    view! {
        <span class={ class_names!("badge", class) }>{ label.to_string() }</span>
    }
}
