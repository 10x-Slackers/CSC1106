use crate::entities::{customer, invoice, user};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Schema, EntityTrait, ActiveValue::
    Set,
};
use std::str::FromStr;

macro_rules! create_tables {
    ($db:expr, $schema:expr, [$($entity:expr),* $(,)?]) => {
        $(
            $db.execute($schema.create_table_from_entity($entity).if_not_exists()).await?;
        )*
    };
}

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new("sqlite://accounting.db?mode=rwc");

    opt.max_connections(5).sqlx_logging(false);

    let db = Database::connect(opt).await?;
    let db_backend = db.get_database_backend();
    let schema: Schema = Schema::new(db_backend);

    db.execute_unprepared("PRAGMA foreign_keys=ON;").await?;
    db.execute_unprepared("PRAGMA journal_mode=WAL;").await?;

    create_tables!(
        db,
        schema,
        [user::Entity, customer::Entity, invoice::Entity]
    );
    if user::Entity::find().one(&db).await?.is_none() {
        seed_db(&db).await?;
    }
    Ok(db)
}

pub async fn seed_db(db: &DatabaseConnection) -> Result<(), DbErr> {
    let admin = user::ActiveModel {
        name: Set("Admin".to_string()),
        email: Set("aa.email.com".to_string()),
        password_hash: Set("hashed_password".to_string()),
        role: Set(user::Role::ADMIN),
        ..Default::default()
    };
    admin.insert(db).await?;
    let customer1 = customer::ActiveModel {
        type_: Set(customer::CustomerType::CUSTOMER),
        name: Set("John Doe".to_string()),
        email: Set("john.email.com".to_string()),
        phone: Set("1234567890".to_string()),
        address: Set("123 Main St".to_string()),
        company: Set(Some("John's Company".to_string())),
        status: Set(customer::CustomerStatus::ACTIVE),
        ..Default::default()
    };
    customer1.insert(db).await?;
    let invoice: invoice::ActiveModel = invoice::ActiveModel {
        customer_id: Set(1),
        created_by: Set(1),
        invoice_number: Set("INV-001".to_string()),
        issue_date: Set(chrono::Utc::now().timestamp()),
        due_date: Set(chrono::Utc::now().timestamp() + 30 * 24 * 60 * 60),
        reference: Set(Some("PO-123".to_string())),
        subtotal: Set(Decimal::from_str("100.50").unwrap()),
        total_amount: Set(Decimal::from_str("107.00").unwrap()),
        paid_amount: Set(Decimal::from_str("0.00").unwrap()),
        status: Set("DRAFT".to_string()),
        ..Default::default()
    };
    invoice.insert(db).await?;
    Ok(())
}
