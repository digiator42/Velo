use std::collections::HashMap;

use crate::components::MetricCard;
use dom::DomNode;
use r#macro::view;
use router::{Router, Route};
use velo_core::Signal;
use web_sys::js_sys;

pub fn monitor_page(_: HashMap<String, String>) -> DomNode {

    // High-Frequency Ticker Signals
    let cpu_load = Signal::new(42);
    let memory_usage = Signal::new(68);
    let active_connections = Signal::new(1205);
    let elapsed_time = Signal::new(0.0);
    let interval = Signal::new(50);
    let cpu_status = Signal::new("NOMINAL".to_string());
    let mem_status = Signal::new("NOMINAL".to_string());
    let conn_status = Signal::new("STABLE".to_string());

    // Manual Interactive Counter Signal
    let manual_clicks = Signal::new(0);

    // Clones for the background streaming interval
    let c_cpu = cpu_load.clone();
    let c_mem = memory_usage.clone();
    let c_conn = active_connections.clone();
    let i_time = elapsed_time.clone();
    let c_interval = interval.clone();
    let s_cpu = cpu_status.clone();
    let s_mem = mem_status.clone();
    let s_conn = conn_status.clone();

    // Clones for the interactive manual button click handler
    let click_counter_view = manual_clicks.clone();
    let click_counter_action = manual_clicks.clone();

    // Background updates every 50ms (20 updates per second!)
    let mut loop_counter = 0;
    gloo_timers::callback::Interval::new(c_interval.get(), move || {
        loop_counter += 1;

        let cpu_delta = 35 + (loop_counter % 30);
        let mem_delta = 60 + (loop_counter % 15);
        let conn_delta = 1100 + (loop_counter % 250);

        let start_time = js_sys::Date::now();

        for _ in 0..1000 {
            c_cpu.set(cpu_delta);
            c_mem.set(mem_delta);
            c_conn.set(conn_delta);
        }

        let end_time = js_sys::Date::now();
        let elapsed = end_time - start_time;

        i_time.set(elapsed);

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
    .forget();

    view! {
        <div class="page monitor">
            <h1>"Live Clusters Performance Metrics"</h1>
            <p>"Demonstrating Velo's non-blocking, fine-grained asynchronous architecture."</p>

            <div class="chaos-controls">
                <div class="interactive-zone">
                    <button class="btn-action" on:click={ move |_| {
                        // Directly mutate manual clicks on a completely separate signal branch
                        click_counter_action.set(click_counter_action.get() + 1);
                    } }>
                        "Smash to Test Responsiveness"
                    </button>
                    <span class="counter-value">{ click_counter_view.get() }</span>
                </div>

                <div class="test-label">
                    <strong>"Browser Thread Status: "</strong>
                    "If the loop below were blocking, this loading circle would stutte, Look at how perfectly smooth it spins!"
                </div>
                <div class="spinner-tester"></div>
            </div>

            <div class="dashboard-grid">
                <MetricCard title={ "Processor Compute Cluster".into() } value={ cpu_load } unit={ "%".into() } status={ cpu_status } />
                <MetricCard title={ "Memory Allocation Pool".into() } value={ memory_usage } unit={ "%".into() } status={ mem_status } />
                <MetricCard title={ "Active Concurrent Sockets".into() } value={ active_connections } unit={ "REQ/s".into() } status={ conn_status } />
            </div>

            <div>
                <h2>"Performance Inspection"</h2>
                <p>"The elapsed time for the 1000 rapid state updates in the background loop is: "
                <span class="elapsed-time">{ elapsed_time.get() }</span> " milliseconds, with interval: "
                <span class="interval">{ c_interval.get() }</span> " ms."</p>
            </div>

            <p class="inspection-note">
                <span class="test-label">"Inspect the DOM: "</span>
                <span class="test-value">"Open Chrome/Firefox DevTools. "</span>
                <span>"Only the inner text contents are flashing. "</span>
                <span>"The wrappers, the dashboard grid, the headers, and the buttons are completely static."</span>
            </p>
        </div>
    }
}
