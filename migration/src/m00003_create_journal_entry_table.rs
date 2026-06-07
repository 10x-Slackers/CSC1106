use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JournalEntry::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JournalEntry::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JournalEntry::PaymentId).integer().null())
                    .col(ColumnDef::new(JournalEntry::ClaimId).integer().null())
                    .col(
                        ColumnDef::new(JournalEntry::CreatedByUserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntry::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntry::Table, JournalEntry::CreatedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalEntry::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum JournalEntry {
    Table,
    Id,
    PaymentId,
    ClaimId,
    CreatedByUserId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
