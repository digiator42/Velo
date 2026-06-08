use core::Signal;

use dom::DomNode;
use r#macro::view;

#[allow(non_snake_case)]
pub fn MetricCard(
    title: String,
    value: Signal<i32>,
    unit: String,
    status: Signal<String>,
) -> DomNode {
    view! {
        <div class="metric-card">
            <h3>{ title.clone() }</h3>
            <div class="metric-display">
                <span class="value">{ value.get() }</span>
                <span class="unit">{ unit.clone() }</span>
            </div>
            <p class="status-indicator">
                "System State: " <span class="status-badge">{ status.get() }</span>
            </p>
        </div>
    }
}
