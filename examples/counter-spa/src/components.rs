use r#velo_macro::view;
use velo_core::create_signal;
use velo_core::ReadSignal;
use velo_dom::DomNode;

#[allow(non_snake_case)]
pub fn UserCard(name: ReadSignal<String>, role: String) -> DomNode {
    view! {
        <div class="user-card">
            <p>"Hello, " { name } "!"</p>
            <p class="role"> "Role: " { role }</p>
        </div>
    }
}
