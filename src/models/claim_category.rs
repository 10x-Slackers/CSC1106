//! <Summary of file>
//!
//! Authors: Teo Kai Wen

use std::collections::HashMap;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::entity::claim_category;
use crate::models::error::AppError;

/// Build a map of category ID → name for the given IDs.
pub async fn name_map_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: Vec<i32>,
) -> Result<HashMap<i32, String>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i32, String)> = claim_category::Entity::find()
        .select_only()
        .column(claim_category::Column::Id)
        .column(claim_category::Column::Name)
        .filter(claim_category::Column::Id.is_in(ids))
        .into_tuple()
        .all(db)
        .await?;
    Ok(rows.into_iter().collect())
}

#[derive(serde::Serialize)]
pub struct CategoryOption {
    pub id: i32,
    pub name: String,
}

/// List all categories as options, ordered by name ascending.
pub async fn list_options<C: ConnectionTrait>(db: &C) -> Result<Vec<CategoryOption>, AppError> {
    let cats = claim_category::Entity::find()
        .order_by_asc(claim_category::Column::Name)
        .all(db)
        .await?;
    Ok(cats
        .into_iter()
        .map(|c| CategoryOption {
            id: c.id,
            name: c.name,
        })
        .collect())
}

/// Check whether a category with the given ID exists.
pub async fn exists<C: ConnectionTrait>(db: &C, id: i32) -> Result<bool, AppError> {
    Ok(claim_category::Entity::find_by_id(id)
        .one(db)
        .await?
        .is_some())
}
