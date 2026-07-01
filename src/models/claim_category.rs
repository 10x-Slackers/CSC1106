use std::collections::HashMap;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::entity::claim_category;
use crate::models::error::AppError;

/// Look up a category name by ID. Returns `None` if the category does not exist.
pub async fn find_name_by_id<C: ConnectionTrait>(
    db: &C,
    id: i32,
) -> Result<Option<String>, AppError> {
    let category = claim_category::Entity::find_by_id(id).one(db).await?;
    Ok(category.map(|c| c.name))
}

/// Build a map of category ID → name for the given IDs.
pub async fn name_map_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: Vec<i32>,
) -> Result<HashMap<i32, String>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let categories = claim_category::Entity::find()
        .filter(claim_category::Column::Id.is_in(ids))
        .all(db)
        .await?;
    Ok(categories.into_iter().map(|c| (c.id, c.name)).collect())
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
