use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // invoice table
        manager
            .create_table(
                Table::create()
                    .table(Invoice::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Invoice::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Invoice::PartyId).integer().not_null())
                    .col(
                        ColumnDef::new(Invoice::CreatedByUserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Invoice::InvoiceNo)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Invoice::IssueDate).date().not_null())
                    .col(ColumnDef::new(Invoice::DueDate).date().not_null())
                    .col(ColumnDef::new(Invoice::Status).string().not_null().extra("CONSTRAINT chk_invoice_status CHECK (status IN ('DRAFT', 'SENT', 'PARTIALLY_PAID', 'PAID', 'VOIDED'))"))
                    .col(ColumnDef::new(Invoice::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Invoice::UpdatedAt).date_time().not_null())
                    .extra("CONSTRAINT chk_invoice_dates CHECK (due_date >= issue_date)")
                    .foreign_key(
                        ForeignKey::create()
                            .from(Invoice::Table, Invoice::PartyId)
                            .to(Party::Table, Party::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Invoice::Table, Invoice::CreatedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // invoice_line_item table
        manager
            .create_table(
                Table::create()
                    .table(InvoiceLineItem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InvoiceLineItem::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InvoiceLineItem::InvoiceId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InvoiceLineItem::Description)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InvoiceLineItem::Quantity)
                            .integer()
                            .not_null()
                            .extra("CONSTRAINT chk_line_item_quantity_positive CHECK (quantity > 0)"),
                    )
                    .col(
                        ColumnDef::new(InvoiceLineItem::UnitPrice)
                            .decimal_len(15, 4)
                            .not_null()
                            .extra("CONSTRAINT chk_line_item_unit_price_nonneg CHECK (unit_price >= 0)"),
                    )
                    .col(ColumnDef::new(InvoiceLineItem::GstRate).string().not_null().extra("CONSTRAINT chk_line_item_gst_rate CHECK (gst_rate IN ('NONE', 'STANDARD'))"))
                    .foreign_key(
                        ForeignKey::create()
                            .from(InvoiceLineItem::Table, InvoiceLineItem::InvoiceId)
                            .to(Invoice::Table, Invoice::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_invoice_line_item_invoice_id")
                    .table(InvoiceLineItem::Table)
                    .col(InvoiceLineItem::InvoiceId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_invoice_line_item_invoice_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(InvoiceLineItem::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Invoice::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
enum Invoice {
    Table,
    Id,
    PartyId,
    CreatedByUserId,
    InvoiceNo,
    IssueDate,
    DueDate,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum InvoiceLineItem {
    Table,
    Id,
    InvoiceId,
    Description,
    Quantity,
    UnitPrice,
    GstRate,
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
