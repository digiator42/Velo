//! `examples/velocity` — a Trello/Linear-lite project management dashboard.
//!
//! One codebase exercising every Velo feature:
//!
//! - `signal!`, `memo!`, `effect!` — live search, filtered counts, theme toggle
//! - `signal_vec!` + keyed `for` — boards/columns/tasks, reconciled by key
//! - async `() => {}` arrows — create/edit/delete actions
//! - `create_resource` + `<Suspense>` — load projects, members, activity feed
//! - `class_names!` — task priority, status badges, dark/light theme
//! - `bind:value` / `bind:checked` — create-task modal, inline edit, filters
//! - `on:submit` — quick-add task form
//! - `class:` toggles — sidebar collapse, selected row, overdue highlight
//! - `app!` file routing — `/`, `/board/:id`, `/board/:id/task/:taskId`, `/settings`, 404
//! - `paths::` typed `<Link>` — compile-checked navigation
//! - `<Link prefetch>` — board list to board detail
//! - `<Head>` metadata — per-route titles
//! - `<ErrorBoundary>` + `boundary_fault` — graceful "task not found" fallback
//! - `use_dynamic` — heavy chart component lazy-loaded on the dashboard
//! - `provide!` / `context!` — auth state, theme, current user
//! - `route_path!` — programmatic navigation after create
//!
//! Mock data: a `MockApi` module (no server) with simulated latency so prefetch
//! promise sharing is visible in the Network tab.

use velo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

velo::app!();

mod api;
mod components;

use crate::api::MockApi;

// ---- Global app state provided via context (auth + theme) ----

#[derive(Clone)]
pub struct AuthState {
    pub user: RwSignal<crate::api::User>,
    pub is_authed: RwSignal<bool>,
}

#[derive(Clone)]
pub struct ThemeState {
    pub dark: RwSignal<bool>,
    pub collapsed: RwSignal<bool>,
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    run_app();
}

pub fn run_app() {
    web_sys::console::log_1(&"[velocity] run_app starting...".into());
    // Seed global context (auth + theme) before mounting so every route and
    // descendant component can read it via `context!()` / `use_context`.
    let users = MockApi::users();
    let current_user = users.into_iter().next().unwrap_or(crate::api::User {
        id: "anon".into(),
        name: "Anonymous".into(),
        avatar: "AN".into(),
    });
    provide!(AuthState {
        user: signal(current_user),
        is_authed: signal(true),
    });
    provide!(ThemeState {
        dark: signal(false),
        collapsed: signal(false),
    });

    let shell = view! {
        <Router routes={ velo_app::routes() } />
    };
    mount(shell);
    web_sys::console::log_1(&"[velocity] mount() done".into());
}
