use velo_core::Signal;

use r#velo_macro::view;
use velo_dom::DomNode;

#[allow(non_snake_case)]
pub fn UserCard(name: Signal<String>, role: String) -> DomNode {
    view! {
        <div class="user-card">
            <p>"Hello, " { name.get() } "!"</p>
            <p class="role"> "Role: " { role.clone() }</p>
        </div>
    }
}
