use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntry::Table)
                    .add_column(ColumnDef::new(JournalEntry::InvoiceId).integer().null())
                    .to_owned(),
            )
            .await?;

        // Foreign key for invoice_id will be added when the invoice table is created
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntry::Table)
                    .drop_column(JournalEntry::InvoiceId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum JournalEntry {
    Table,
    InvoiceId,
}
