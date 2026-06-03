// TODO: Replace this with a real database model
pub struct User {
    pub username:     &'static str,
    pub password:     &'static str,
    pub display_name: &'static str,
}

// TODO: Replace this with a real database lookup
pub fn find_user(username: &str) -> Option<&'static User> {
    static USERS: &[User] = &[
        User {
            username:     "admin",
            password:     "admin123",
            display_name: "Administrator",
        },
        User {
            username:     "alice",
            password:     "alice123",
            display_name: "Alice (Accountant)",
        },
    ];

    USERS.iter().find(|u| u.username == username)
}
