use crate::components::MetricCard;
use core::Signal;
use dom::DomNode;
use r#macro::view;

pub fn monitor_page() -> DomNode {
    // Initialize our fine-grained reactive core signals
    let cpu_load = Signal::new(42);
    let memory_usage = Signal::new(68);
    let active_connections = Signal::new(1205);

    let cpu_status = Signal::new("NOMINAL".to_string());
    let mem_status = Signal::new("NOMINAL".to_string());
    let conn_status = Signal::new("STABLE".to_string());

    // Clones for our asynchronous interval loop
    let c_cpu = cpu_load.clone();
    let c_mem = memory_usage.clone();
    let c_conn = active_connections.clone();
    let s_cpu = cpu_status.clone();
    let s_mem = mem_status.clone();
    let s_conn = conn_status.clone();

    let mut loop_counter = 0;

    gloo_timers::callback::Interval::new(60, move || {
        loop_counter += 1;

        let cpu_delta = 35 + (loop_counter % 30);
        let mem_delta = 60 + (loop_counter % 15);
        let conn_delta = 1100 + (loop_counter % 250);

        // Update the reactive signal values
        c_cpu.set(cpu_delta);
        c_mem.set(mem_delta);
        c_conn.set(conn_delta);

        if cpu_delta > 60 {
            s_cpu.set("HIGH LOAD".to_string());
        } else {
            s_cpu.set("NOMINAL".to_string());
        }

        if mem_delta > 72 {
            s_mem.set("WARNING".to_string());
        } else {
            s_mem.set("NOMINAL".to_string());
        }

        if conn_delta > 1300 {
            s_conn.set("PEAK SURGE".to_string());
        } else {
            s_conn.set("STABLE".to_string());
        }
    })
    .forget(); // .forget() keeps the interval detached and spinning for the view lifetime

    view! {
        <div class="page monitor">
            <h1>"Live Clusters Performance Metrics"</h1>
            <p>"The cards below receive updates directly from an automated streaming runtime event loop."</p>

            <div class="dashboard-grid">
                <MetricCard title={ "Processor Compute Cluster".into() } value={ cpu_load } unit={ "%".into() } status={ cpu_status } />
                <MetricCard title={ "Memory Allocation Pool".into() } value={ memory_usage } unit={ "%".into() } status={ mem_status } />
                <MetricCard title={ "Active Concurrent Sockets".into() } value={ active_connections } unit={ "REQ/s".into() } status={ conn_status } />
            </div>
        </div>
    }
}
