use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::entity::party as party_entity;
use crate::entity::party::{PartyStatus, PartyType};

impl PartyType {
    /// Parses a string into a [`PartyType`].
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("customer") {
            Some(PartyType::Customer)
        } else if s.eq_ignore_ascii_case("vendor") {
            Some(PartyType::Vendor)
        } else {
            None
        }
    }
}

impl PartyStatus {
    /// Parses a string into a [`PartyStatus`].
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("active") {
            Some(PartyStatus::Active)
        } else if s.eq_ignore_ascii_case("inactive") {
            Some(PartyStatus::Inactive)
        } else {
            None
        }
    }
}
/// Party Model to define a Party Record for Application-Level Use.
#[derive(Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: i32,
    pub party_type: PartyType,
    pub name: String,
    pub company: Option<String>,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub status: PartyStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<party_entity::Model> for Party {
    /// Converts a SeaORM party entity model into an application [`Party`].
    fn from(m: party_entity::Model) -> Self {
        Party {
            id: m.id,
            party_type: m.party_type,
            name: m.name,
            company: m.company,
            email: m.email,
            phone: m.phone,
            address: m.address,
            status: m.status,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
