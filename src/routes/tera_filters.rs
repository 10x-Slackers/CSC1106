use std::collections::HashMap;

use chrono::NaiveDateTime;
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

/// Tera filter that formats a `NaiveDateTime` string into a human-readable form. Defaults to `"%Y-%m-%d %H:%M"`.
pub fn datetime(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let Some(s) = value.as_str() else {
        return Ok(value.clone());
    };
    let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") else {
        return Ok(value.clone());
    };
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d %H:%M");
    Ok(Value::String(dt.format(format).to_string()))
}
