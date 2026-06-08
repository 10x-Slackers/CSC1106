use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Claim::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Claim::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Claim::SubmittedByUserId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Claim::ReviewedByUserId).integer().null())
                    .col(ColumnDef::new(Claim::CategoryId).integer().not_null())
                    .col(ColumnDef::new(Claim::Title).string().not_null())
                    .col(ColumnDef::new(Claim::Description).text().not_null())
                    .col(ColumnDef::new(Claim::Amount).decimal_len(15, 4).not_null().extra("CONSTRAINT chk_claim_amount_positive CHECK (amount > 0)"))
                    .col(ColumnDef::new(Claim::PurchaseDate).date().not_null())
                    .col(ColumnDef::new(Claim::Status).string().not_null().extra("CONSTRAINT chk_claim_status CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED'))"))
                    .col(ColumnDef::new(Claim::RejectionReason).text().null())
                    .col(ColumnDef::new(Claim::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Claim::UpdatedAt).date_time().not_null())
                    .check(Expr::cust("status != 'REJECTED' OR rejection_reason IS NOT NULL"))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Claim::Table, Claim::SubmittedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Claim::Table, Claim::ReviewedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Claim::Table, Claim::CategoryId)
                            .to(ClaimCategory::Table, ClaimCategory::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Claim::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
enum Claim {
    Table,
    Id,
    SubmittedByUserId,
    ReviewedByUserId,
    CategoryId,
    Title,
    Description,
    Amount,
    PurchaseDate,
    Status,
    RejectionReason,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ClaimCategory {
    Table,
    Id,
}
