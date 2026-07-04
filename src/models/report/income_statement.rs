use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;

use crate::entity::account::AccountCategory;
use crate::models::account::balances_by_category;
use crate::models::error::AppError;

#[derive(serde::Serialize)]
pub struct IncomeStatementLine {
    pub account_name: String,
    pub amount: Decimal,
}

#[derive(serde::Serialize)]
pub struct IncomeStatement {
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub revenue: Vec<IncomeStatementLine>,
    pub total_revenue: Decimal,
    pub expenses: Vec<IncomeStatementLine>,
    pub total_expenses: Decimal,
    pub net_profit: Decimal,
    pub is_loss: bool,
}

pub struct IncomeStatementReport;

impl IncomeStatementReport {
    /// Compute total revenue and total expenses for the date range, then subtract to get profit or loss.
    pub async fn compute(
        db: &DatabaseConnection,
        from: Option<NaiveDateTime>,
        to: Option<NaiveDateTime>,
    ) -> Result<IncomeStatement, AppError> {
        let (revenue_balances, expense_balances) = futures::try_join!(
            balances_by_category(db, &[AccountCategory::Revenue], from, to),
            balances_by_category(db, &[AccountCategory::Expense], from, to),
        )?;

        let revenue: Vec<IncomeStatementLine> = revenue_balances
            .into_iter()
            .map(|ab| IncomeStatementLine {
                account_name: ab.account.name,
                amount: ab.balance,
            })
            .collect();

        let expenses: Vec<IncomeStatementLine> = expense_balances
            .into_iter()
            .map(|ab| IncomeStatementLine {
                account_name: ab.account.name,
                amount: ab.balance,
            })
            .collect();

        let total_revenue: Decimal = revenue.iter().map(|l| l.amount).sum();
        let total_expenses: Decimal = expenses.iter().map(|l| l.amount).sum();
        let net_profit = total_revenue - total_expenses;
        let is_loss = net_profit < Decimal::ZERO;

        Ok(IncomeStatement {
            from,
            to,
            revenue,
            total_revenue,
            expenses,
            total_expenses,
            net_profit,
            is_loss,
        })
    }
}
