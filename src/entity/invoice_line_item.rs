use rust_decimal::Decimal;
use sea_orm::Iterable;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// GST rate applied to an invoice line item.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum GstRate {
    #[sea_orm(string_value = "NONE")]
    None,
    #[sea_orm(string_value = "STANDARD")]
    Standard,
}

impl fmt::Display for GstRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GstRate::None => write!(f, "None"),
            GstRate::Standard => write!(f, "Standard"),
        }
    }
}

impl GstRate {
    /// Return all GST rate variant labels in declaration order.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }

    /// Parse a GST rate string (case-insensitive) into a `GstRate`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "none" => Some(GstRate::None),
            "standard" => Some(GstRate::Standard),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "invoice_line_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub invoice_id: i32,
    pub description: String,
    pub quantity: i32,
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub unit_price: Decimal,
    pub gst_rate: GstRate,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::invoice::Column::Id"
    )]
    Invoice,
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
