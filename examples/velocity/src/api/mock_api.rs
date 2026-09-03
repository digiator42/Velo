use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub title: String,
    pub order: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Status {
    Todo,
    InProgress,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub column_id: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub status: Status,
    pub assignee: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub action: String,
    pub user: String,
    pub target: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

/// Aggregate counts the dashboard renders (and the lazy-loaded `Chart` consumes).
#[derive(Clone, Debug, Default)]
pub struct DashboardStats {
    pub projects: usize,
    pub total_tasks: usize,
    pub done_tasks: usize,
    pub in_progress: usize,
    pub overdue: usize,
    pub completion: String,
}

pub struct MockApi;

thread_local! {
    static DB: std::cell::RefCell<MockDb> = std::cell::RefCell::new(MockDb::new());
}

struct MockDb {
    projects: HashMap<String, Project>,
    columns: HashMap<String, Vec<Column>>,
    tasks: HashMap<String, Vec<Task>>,
    activities: Vec<Activity>,
    users: Vec<User>,
}

impl MockDb {
    fn new() -> Self {
        let users = vec![
            User { id: "u1".into(), name: "Alice Chen".into(), avatar: "AC".into() },
            User { id: "u2".into(), name: "Bob Smith".into(), avatar: "BS".into() },
            User { id: "u3".into(), name: "Carol Davis".into(), avatar: "CD".into() },
        ];

        let projects = vec![
            Project { id: "p1".into(), name: "Website Redesign".into(), description: "Complete overhaul of the company website".into() },
            Project { id: "p2".into(), name: "Mobile App".into(), description: "iOS and Android app development".into() },
            Project { id: "p3".into(), name: "API Platform".into(), description: "Backend API infrastructure".into() },
        ];

        let mut projects_map: HashMap<String, Project> = HashMap::new();
        for p in &projects {
            projects_map.insert(p.id.clone(), p.clone());
        }

        let columns: HashMap<String, Vec<Column>> = [
            ("p1".to_string(), vec![
                Column { id: "p1c1".into(), title: "Backlog".into(), order: 0 },
                Column { id: "p1c2".into(), title: "In Progress".into(), order: 1 },
                Column { id: "p1c3".into(), title: "Review".into(), order: 2 },
                Column { id: "p1c4".into(), title: "Done".into(), order: 3 },
            ]),
            ("p2".to_string(), vec![
                Column { id: "p2c1".into(), title: "Backlog".into(), order: 0 },
                Column { id: "p2c2".into(), title: "In Progress".into(), order: 1 },
                Column { id: "p2c3".into(), title: "Done".into(), order: 2 },
            ]),
            ("p3".to_string(), vec![
                Column { id: "p3c1".into(), title: "Planning".into(), order: 0 },
                Column { id: "p3c2".into(), title: "Development".into(), order: 1 },
                Column { id: "p3c3".into(), title: "Deployed".into(), order: 2 },
            ]),
        ].into_iter().collect();

        let now = js_sys::Date::new_0();
        let now_iso = now.to_iso_string().as_string().unwrap_or_default();

        let tasks: HashMap<String, Vec<Task>> = [
            ("p1".to_string(), vec![
                Task { id: "p1t1".into(), project_id: "p1".into(), column_id: "p1c1".into(), title: "Design homepage mockup".into(), description: "Create wireframes and high-fidelity designs for the new homepage.".into(), priority: Priority::High, status: Status::Todo, assignee: "Alice Chen".into(), created_at: now_iso.clone() },
                Task { id: "p1t2".into(), project_id: "p1".into(), column_id: "p1c1".into(), title: "Update brand colors".into(), description: "Apply the new brand palette across all pages.".into(), priority: Priority::Medium, status: Status::Todo, assignee: "Bob Smith".into(), created_at: now_iso.clone() },
                Task { id: "p1t3".into(), project_id: "p1".into(), column_id: "p1c2".into(), title: "Build responsive navbar".into(), description: "Implement the mobile-friendly navigation component.".into(), priority: Priority::High, status: Status::InProgress, assignee: "Carol Davis".into(), created_at: now_iso.clone() },
                Task { id: "p1t4".into(), project_id: "p1".into(), column_id: "p1c3".into(), title: "Hero section redesign".into(), description: "New hero with animated gradient background.".into(), priority: Priority::Medium, status: Status::Done, assignee: "Alice Chen".into(), created_at: now_iso.clone() },
                Task { id: "p1t5".into(), project_id: "p1".into(), column_id: "p1c4".into(), title: "Footer restructure".into(), description: "New footer with links, social icons, and newsletter signup.".into(), priority: Priority::Low, status: Status::Done, assignee: "Bob Smith".into(), created_at: now_iso.clone() },
            ]),
            ("p2".to_string(), vec![
                Task { id: "p2t1".into(), project_id: "p2".into(), column_id: "p2c1".into(), title: "Set up React Native project".into(), description: "Initialize the monorepo with shared components.".into(), priority: Priority::High, status: Status::InProgress, assignee: "Carol Davis".into(), created_at: now_iso.clone() },
                Task { id: "p2t2".into(), project_id: "p2".into(), column_id: "p2c2".into(), title: "Implement push notifications".into(), description: "Firebase Cloud Messaging integration for iOS and Android.".into(), priority: Priority::Medium, status: Status::Todo, assignee: "Alice Chen".into(), created_at: now_iso.clone() },
                Task { id: "p2t3".into(), project_id: "p2".into(), column_id: "p2c3".into(), title: "User authentication flow".into(), description: "Login, signup, and password reset screens.".into(), priority: Priority::High, status: Status::Done, assignee: "Bob Smith".into(), created_at: now_iso.clone() },
            ]),
            ("p3".to_string(), vec![
                Task { id: "p3t1".into(), project_id: "p3".into(), column_id: "p3c1".into(), title: "Design API schema".into(), description: "REST and GraphQL endpoints for the v1 API.".into(), priority: Priority::High, status: Status::InProgress, assignee: "Alice Chen".into(), created_at: now_iso.clone() },
                Task { id: "p3t2".into(), project_id: "p3".into(), column_id: "p3c2".into(), title: "Implement auth middleware".into(), description: "JWT token validation and refresh logic.".into(), priority: Priority::High, status: Status::Todo, assignee: "Bob Smith".into(), created_at: now_iso.clone() },
                Task { id: "p3t3".into(), project_id: "p3".into(), column_id: "p3c2".into(), title: "Database migrations".into(), description: "PostgreSQL schema with Prisma ORM.".into(), priority: Priority::Medium, status: Status::Todo, assignee: "Carol Davis".into(), created_at: now_iso.clone() },
            ]),
        ].into_iter().collect();

        let activities = vec![
            Activity { id: "a1".into(), action: "moved".into(), user: "Alice Chen".into(), target: "\"Hero section redesign\" to Review".into(), timestamp: now_iso.clone() },
            Activity { id: "a2".into(), action: "created".into(), user: "Bob Smith".into(), target: "\"Database migrations\" in API Platform".into(), timestamp: now_iso.clone() },
            Activity { id: "a3".into(), action: "completed".into(), user: "Carol Davis".into(), target: "\"Footer restructure\"".into(), timestamp: now_iso.clone() },
            Activity { id: "a4".into(), action: "commented on".into(), user: "Alice Chen".into(), target: "\"Design homepage mockup\"".into(), timestamp: now_iso.clone() },
            Activity { id: "a5".into(), action: "moved".into(), user: "Bob Smith".into(), target: "\"Implement push notifications\" to In Progress".into(), timestamp: now_iso.clone() },
        ];

        Self {
            projects: projects_map,
            columns,
            tasks,
            activities,
            users,
        }
    }
}

impl MockApi {
    pub fn projects() -> Vec<Project> {
        DB.with(|db| db.borrow().projects.values().cloned().collect())
    }

    pub fn project(id: &str) -> Option<Project> {
        DB.with(|db| db.borrow().projects.get(id).cloned())
    }

    pub fn columns(project_id: &str) -> Vec<Column> {
        DB.with(|db| {
            let mut cols = db.borrow()
                .columns
                .get(project_id)
                .cloned()
                .unwrap_or_default();
            cols.sort_by_key(|c| c.order);
            cols
        })
    }

    pub fn tasks(project_id: &str) -> Vec<Task> {
        DB.with(|db| db.borrow().tasks.get(project_id).cloned().unwrap_or_default())
    }

    pub fn task(project_id: &str, task_id: &str) -> Option<Task> {
        DB.with(|db| {
            db.borrow().tasks.get(project_id)?
                .iter().find(|t| t.id == task_id).cloned()
        })
    }

    pub fn tasks_in_column(project_id: &str, column_id: &str) -> Vec<Task> {
        DB.with(|db| {
            db.borrow().tasks.get(project_id)
                .map(|tasks| tasks.iter().filter(|t| t.column_id == column_id).cloned().collect())
                .unwrap_or_default()
        })
    }

    pub fn activities() -> Vec<Activity> {
        DB.with(|db| db.borrow().activities.clone())
    }

    pub fn users() -> Vec<User> {
        DB.with(|db| db.borrow().users.clone())
    }

    pub fn move_task(project_id: &str, task_id: &str, new_column_id: &str, new_status: Status) {
        DB.with(|db| {
            let mut db = db.borrow_mut();
            if let Some(tasks) = db.tasks.get_mut(project_id) {
                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                    task.column_id = new_column_id.into();
                    task.status = new_status;
                }
            }
        });
    }

    pub fn create_task(project_id: &str, column_id: &str, title: &str, priority: Priority) -> Task {
        let now = js_sys::Date::new_0();
        let now_iso = now.to_iso_string().as_string().unwrap_or_default();
        let id = format!("t-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let task = Task {
            id,
            project_id: project_id.into(),
            column_id: column_id.into(),
            title: title.into(),
            description: String::new(),
            priority,
            status: Status::Todo,
            assignee: String::new(),
            created_at: now_iso,
        };
        DB.with(|db| {
            let mut db = db.borrow_mut();
            db.tasks.entry(project_id.into()).or_default().push(task.clone());
        });
        task
    }

    pub fn delete_task(project_id: &str, task_id: &str) {
        DB.with(|db| {
            let mut db = db.borrow_mut();
            if let Some(tasks) = db.tasks.get_mut(project_id) {
                tasks.retain(|t| t.id != task_id);
            }
        });
    }

    /// Naive "overdue" check: a task is overdue if it was created more than
    /// 48 hours ago and is not yet Done. Used by `TaskCard`'s `class:overdue`
    /// toggle to demonstrate reactive class toggles driven by a method.
    pub fn is_overdue(task: &Task) -> bool {
        if task.status == Status::Done {
            return false;
        }
        let created_ms = js_sys::Date::parse(&task.created_at);
        if created_ms.is_nan() {
            return false;
        }
        let age_ms = js_sys::Date::now() - created_ms;
        age_ms > 48.0 * 3600.0 * 1000.0
    }

    // ---- Aggregated stats helper for the dashboard ----

    /// A small snapshot of aggregate counts the dashboard renders (and that
    /// the lazy-loaded `Chart` component consumes).
    pub fn dashboard_stats() -> DashboardStats {
        let projects = MockApi::projects();
        let mut total = 0usize;
        let mut done = 0usize;
        let mut in_progress = 0usize;
        let mut overdue = 0usize;
        for proj in &projects {
            let tasks = MockApi::tasks(&proj.id);
            total += tasks.len();
            for t in &tasks {
                if t.status == Status::Done { done += 1; }
                if t.status == Status::InProgress { in_progress += 1; }
                if MockApi::is_overdue(&t) { overdue += 1; }
            }
        }
        let completion = if total > 0 {
            format!("{}%", done * 100 / total)
        } else {
            "—".to_string()
        };
        DashboardStats {
            projects: projects.len(),
            total_tasks: total,
            done_tasks: done,
            in_progress,
            overdue,
            completion,
        }
    }
}

/// Simulate network latency for the mock API. Used by `create_resource` callers
/// to make loading states visible (and to give prefetch a window to win the race).
pub async fn mock_delay_ms(ms: u32) {
    velo::sleep(ms).await;
}
