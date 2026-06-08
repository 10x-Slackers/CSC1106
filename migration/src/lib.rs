pub use sea_orm_migration::prelude::*;

mod m00001_create_user_table;
mod m00002_create_account_table;
mod m00003_create_journal_entry_table;
mod m00004_create_journal_entry_line_table;
mod m00005_add_journal_entry_invoice_id;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_user_table::Migration),
            Box::new(m00002_create_account_table::Migration),
            Box::new(m00003_create_journal_entry_table::Migration),
            Box::new(m00004_create_journal_entry_line_table::Migration),
            Box::new(m00005_add_journal_entry_invoice_id::Migration),
        ]
    }
}
