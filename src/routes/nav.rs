use tera::Context;

use crate::entity::user::Role;
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{AdminOnly, Finance, RoleSet};

/// A navigation link.
#[derive(Clone, serde::Serialize)]
pub struct NavLink {
    pub name: String,
    pub href: String,
}

/// Insert navbar context variables into a Tera context.
pub fn insert_nav_context(context: &mut Context, user: &Authenticated) {
    context.insert("nav_links", &nav_links_for_role(&user.role));
    context.insert("current_user", user);
}

/// Build the navigation links for a given user role.
fn nav_links_for_role(role: &Role) -> Vec<NavLink> {
    // Base links
    let mut links = vec![
        NavLink {
            name: "Dashboard".into(),
            href: "/".into(),
        },
        NavLink {
            name: "Invoices".into(),
            href: "/invoices".into(),
        },
        NavLink {
            name: "Claims".into(),
            href: "/claims".into(),
        },
        NavLink {
            name: "Parties".into(),
            href: "/parties".into(),
        },
    ];

    if Finance::ROLES.contains(role) {
        links.push(NavLink {
            name: "Payments".into(),
            href: "/payments".into(),
        });
        links.push(NavLink {
            name: "Reports".into(),
            href: "/reports".into(),
        });
    }

    if AdminOnly::ROLES.contains(role) {
        links.push(NavLink {
            name: "Users".into(),
            href: "/users".into(),
        });
    }

    links
}
