use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Party::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Party::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Party::PartyType).string().not_null().extra(
                        "CONSTRAINT chk_party_type CHECK (party_type IN ('CUSTOMER', 'VENDOR'))",
                    ))
                    .col(ColumnDef::new(Party::Name).string().not_null())
                    .col(ColumnDef::new(Party::Company).string().null())
                    .col(
                        ColumnDef::new(Party::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Party::Phone).string().not_null())
                    .col(ColumnDef::new(Party::Address).string().not_null())
                    .col(ColumnDef::new(Party::Status).string().not_null().extra(
                        "CONSTRAINT chk_party_status CHECK (status IN ('ACTIVE', 'INACTIVE'))",
                    ))
                    .col(
                        ColumnDef::new(Party::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Party::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Party::Table).if_exists().to_owned())
            .await
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
enum Party {
    Table,
    Id,
    PartyType,
    Name,
    Company,
    Email,
    Phone,
    Address,
    Status,
    CreatedAt,
    UpdatedAt,
}
