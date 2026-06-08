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
                    .col(
                        ColumnDef::new(Party::Type).string().not_null().extra(
                            "CONSTRAINT chk_party_type CHECK (type IN ('CUSTOMER', 'VENDOR'))",
                        ),
                    )
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
                    .col(ColumnDef::new(Party::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Party::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Party::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Party {
    Table,
    Id,
    Type,
    Name,
    Company,
    Email,
    Phone,
    Address,
    Status,
    CreatedAt,
    UpdatedAt,
}
