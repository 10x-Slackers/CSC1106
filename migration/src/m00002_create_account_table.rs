use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Account::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Account::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Account::Category)
                            .string()
                            .not_null()
                            .extra(
                                "CONSTRAINT chk_account_category CHECK (category IN ('ASSET', 'LIABILITY', 'EQUITY', 'REVENUE', 'EXPENSE'))",
                            ),
                    )
                    .col(
                        ColumnDef::new(Account::NormalBalance)
                            .string()
                            .not_null()
                            .extra("CONSTRAINT chk_account_normal_balance CHECK (normal_balance IN ('DEBIT', 'CREDIT'))"),
                    )
                    .col(ColumnDef::new(Account::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Account::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
    Name,
    Category,
    NormalBalance,
    CreatedAt,
}
