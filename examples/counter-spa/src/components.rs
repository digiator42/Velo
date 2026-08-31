use velo::prelude::*;

#[allow(non_snake_case)]
#[component]
pub fn UserCard(name: ReadSignal<String>, role: String) -> DomNode {
    view! {
        <div class="user-card">
            <p>"Hello, " { name } "!"</p>
            <p class="role"> "Role: " { role }</p>
        </div>
    }
}
