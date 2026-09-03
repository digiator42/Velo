
/// Reusable presentational + stateful components for the Velocity dashboard.
///
/// Each is annotated with `#[component]`, which synthesises a `<Name>Props`
/// struct with one `pub` field per parameter. Components are matched by name
/// in `view!`, so they can be used in any order once imported:
///
/// ```ignore
/// use crate::components::*;
/// ```
pub mod activity_feed;
pub mod board_header;
pub mod chart;
pub mod create_task_modal;
pub mod kanban_board;
pub mod kanban_column;
pub mod keyboard_shortcuts;
pub mod priority_badge;
pub mod stats_card;
pub mod status_badge;
pub mod task_card;
pub mod task_detail;
pub mod theme_toggle;

// Glob re-export: a `view!` references both the component function and its
// generated `<Name>Props` struct, so both must be in scope. A single
// `use crate::components::*;` in any page brings them all in.
pub use activity_feed::*;
pub use board_header::*;
pub use chart::*;
pub use create_task_modal::*;
pub use kanban_board::*;
pub use kanban_column::*;
pub use keyboard_shortcuts::*;
pub use priority_badge::*;
pub use stats_card::*;
pub use status_badge::*;
pub use task_card::*;
pub use task_detail::*;
pub use theme_toggle::*;