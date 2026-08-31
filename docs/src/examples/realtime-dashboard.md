# Example: High-Frequency Live Dashboard

Demonstrates Velo's non-blocking, fine-grained architecture under high-frequency background streaming without Virtual DOM bottlenecks.

---

## 1. High-Frequency Metrics Component

```rust
use velo::prelude::*;

#[allow(non_snake_case)]
#[component]
pub fn MetricCard(title: String, value: RwSignal<i32>, unit: String) {
    view! {
        <div class="metric-card">
            <h3>{ title }</h3>
            <div class="display">
                <span class="value">{ value }</span>
                <span class="unit">{ unit }</span>
            </div>
        </div>
    }
}
```

---

## 2. Background Stream Loop & Smooth Thread Prover

```rust
use velo::prelude::*;

#[component]
pub fn DashboardPage() {
    let cpu_load = signal(35);
    let memory_usage = signal(60);
    let active_connections = signal(1200);

    let cpu = cpu_load.clone();
    let mem = memory_usage.clone();
    let conn = active_connections.clone();

    // Stream 20 updates per second
    let mut ticker = 0;
    gloo_timers::callback::Interval::new(50, move || {
        ticker += 1;
        cpu.set(30 + (ticker % 40));
        mem.set(50 + (ticker % 30));
        conn.set(1000 + (ticker % 500));
    }).forget();

    view! {
        <div class="dashboard">
            <h1>"Real-Time Server Monitor"</h1>
            
            <div class="grid">
                <MetricCard title="CPU Compute".into() value={ cpu_load } unit="%".into() />
                <MetricCard title="Memory Pool".into() value={ memory_usage } unit="%".into() />
                <MetricCard title="Active Sockets".into() value={ active_connections } unit="req/s".into() } />
            </div>

            <!-- CSS Spinner proves browser thread is never blocked by VDOM diffing -->
            <div class="spinner-tester"></div>
        </div>
    }
}
```
