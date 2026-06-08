use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Payment::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Payment::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Payment::InvoiceId).integer().null())
                    .col(ColumnDef::new(Payment::PartyId).integer().null())
                    .col(
                        ColumnDef::new(Payment::CreatedByUserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Payment::PaymentDirection)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Payment::Amount)
                            .decimal_len(15, 4)
                            .not_null()
                            .extra("CONSTRAINT chk_payment_amount_positive CHECK (amount > 0)"),
                    )
                    .col(ColumnDef::new(Payment::PaymentDate).date().not_null())
                    .col(ColumnDef::new(Payment::Remarks).text().null())
                    .col(ColumnDef::new(Payment::CreatedAt).date_time().not_null())
                    .check(Expr::cust("payment_direction IN ('IN', 'OUT')"))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Payment::Table, Payment::InvoiceId)
                            .to(Invoice::Table, Invoice::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Payment::Table, Payment::PartyId)
                            .to(Party::Table, Party::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Payment::Table, Payment::CreatedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_invoice_id")
                    .table(Payment::Table)
                    .col(Payment::InvoiceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_payment_party_id")
                    .table(Payment::Table)
                    .col(Payment::PartyId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_payment_party_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_payment_invoice_id").to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Payment::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
enum Payment {
    Table,
    Id,
    InvoiceId,
    PartyId,
    CreatedByUserId,
    PaymentDirection,
    Amount,
    PaymentDate,
    Remarks,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Invoice {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Party {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
