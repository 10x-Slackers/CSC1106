use std::collections::HashMap;

use tera::{Result, Value};

use crate::entity::user::Role;
use crate::middleware::permissions::role_set;

/// Tera filter for role-based visibility checks against a named role set.
///
/// Usage: `{% if current_user.role | can(set="finance") %}...{% endif %}`
pub fn can(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let role = value.as_str().and_then(Role::parse);
    let allowed = args.get("set").and_then(|v| v.as_str()).and_then(role_set);
    Ok(Value::Bool(
        role.is_some_and(|r| allowed.is_some_and(|s| s.contains(&r))),
    ))
}
