use velo::prelude::*;

#[page]
pub fn page() -> DomNode {
    view! {
        <div>
            <Head title="Users · typed-nav" />
            <h1>"Users"</h1>
            <ul>
                <li><Link to={ paths::users_id("1") } label="User 1" /></li>
                <li><Link to={ paths::users_id("2") } label="User 2" /></li>
            </ul>
        </div>
    }
}
