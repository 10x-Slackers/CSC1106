use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // journal_entry
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
                    .col(ColumnDef::new(JournalEntry::InvoiceId).integer().null())
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
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntry::Table, JournalEntry::PaymentId)
                            .to(Payment::Table, Payment::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntry::Table, JournalEntry::ClaimId)
                            .to(Claim::Table, Claim::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntry::Table, JournalEntry::InvoiceId)
                            .to(Invoice::Table, Invoice::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // journal_entry_line
        manager
            .create_table(
                Table::create()
                    .table(JournalEntryLine::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JournalEntryLine::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryLine::EntryId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryLine::AccountId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryLine::EntrySide)
                            .string()
                            .not_null()
                            .extra(
                                "CONSTRAINT chk_journal_entry_line_entry_side CHECK (entry_side IN ('DEBIT', 'CREDIT'))",
                            ),
                    )
                    .col(
                        ColumnDef::new(JournalEntryLine::Amount)
                            .decimal_len(15, 4)
                            .not_null()
                            .extra("CONSTRAINT chk_amount_positive CHECK (amount > 0)"),
                    )
                    .col(ColumnDef::new(JournalEntryLine::Description).text().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntryLine::Table, JournalEntryLine::EntryId)
                            .to(JournalEntry::Table, JournalEntry::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntryLine::Table, JournalEntryLine::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_journal_entry_line_entry_id")
                    .table(JournalEntryLine::Table)
                    .col(JournalEntryLine::EntryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_journal_entry_line_account_id")
                    .table(JournalEntryLine::Table)
                    .col(JournalEntryLine::AccountId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_journal_entry_line_account_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_journal_entry_line_entry_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(JournalEntryLine::Table).to_owned())
            .await?;

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
    InvoiceId,
    CreatedByUserId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum JournalEntryLine {
    Table,
    Id,
    EntryId,
    AccountId,
    EntrySide,
    Amount,
    Description,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Payment {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Claim {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Invoice {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
}
