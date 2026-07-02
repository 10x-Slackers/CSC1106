pub use sea_orm_migration::prelude::*;

mod m00001_create_user_table;
mod m00002_create_account_table;
mod m00003_create_claim_category_table;
mod m00004_create_party_table;
mod m00005_create_claim_table;
mod m00006_create_invoice_and_line_item_tables;
mod m00007_create_payment_table;
mod m00008_create_journal_entry_tables;
mod m00009_add_indexes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_user_table::Migration),
            Box::new(m00002_create_account_table::Migration),
            Box::new(m00003_create_claim_category_table::Migration),
            Box::new(m00004_create_party_table::Migration),
            Box::new(m00005_create_claim_table::Migration),
            Box::new(m00006_create_invoice_and_line_item_tables::Migration),
            Box::new(m00007_create_payment_table::Migration),
            Box::new(m00008_create_journal_entry_tables::Migration),
            Box::new(m00009_add_indexes::Migration),
        ]
    }
}
