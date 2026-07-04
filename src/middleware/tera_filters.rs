use std::collections::HashMap;

use chrono::NaiveDateTime;
use tera::{Result, Tera, Value};

use crate::entity::user::Role;
use crate::middleware::permissions::role_set;

/// Showing/hiding template UI based on user role. Usage: `{{ user.role | can(set="finance") }}`
pub fn can(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let role = value.as_str().and_then(Role::parse);
    let allowed = args.get("set").and_then(|v| v.as_str()).and_then(role_set);
    Ok(Value::Bool(
        role.is_some_and(|r| allowed.is_some_and(|s| s.contains(&r))),
    ))
}

/// Formats datetime string into human readable format. Usage: `{{ value | datetime(format="%d %b %Y") }}`
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

/// Formats a decimal string into a money format. Usage: `{{ value | money }}`
pub fn money(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let Some(s) = value.as_str() else {
        return Ok(value.clone());
    };
    match s.parse::<rust_decimal::Decimal>() {
        Ok(d) => Ok(Value::String(format!("S${:.2}", d))),
        Err(_) => Ok(value.clone()),
    }
}

/// Registers custom Tera filters for money formatting, datetime formatting, and role checks.
pub fn register(tera: &mut Tera) {
    tera.register_filter("can", can);
    tera.register_filter("datetime", datetime);
    tera.register_filter("money", money);
}
