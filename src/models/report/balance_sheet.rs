use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;

use crate::entity::account::AccountCategory;
use crate::models::account::balances_by_category;
use crate::models::error::AppError;

#[derive(serde::Serialize)]
pub struct BalanceSheetLine {
    pub account_name: String,
    pub amount: Decimal,
}

#[derive(serde::Serialize)]
pub struct BalanceSheet {
    pub as_of: Option<NaiveDateTime>,
    pub assets: Vec<BalanceSheetLine>,
    pub total_assets: Decimal,
    pub liabilities: Vec<BalanceSheetLine>,
    pub total_liabilities: Decimal,
    pub equity: Vec<BalanceSheetLine>,
    pub total_equity: Decimal,
    pub total_liabilities_and_equity: Decimal,
    pub current_period_net_income: Decimal,
}

pub struct BalanceSheetReport;

impl BalanceSheetReport {
    pub async fn compute(
        db: &DatabaseConnection,
        as_of: Option<NaiveDateTime>,
    ) -> Result<BalanceSheet, AppError> {
        let asset_balances =
            balances_by_category(db, &[AccountCategory::Asset], None, as_of).await?;
        let liability_balances =
            balances_by_category(db, &[AccountCategory::Liability], None, as_of).await?;
        let equity_balances =
            balances_by_category(db, &[AccountCategory::Equity], None, as_of).await?;

        let assets: Vec<BalanceSheetLine> = asset_balances
            .into_iter()
            .map(|ab| BalanceSheetLine {
                account_name: ab.account.name,
                amount: ab.balance,
            })
            .collect();

        let liabilities: Vec<BalanceSheetLine> = liability_balances
            .into_iter()
            .map(|ab| BalanceSheetLine {
                account_name: ab.account.name,
                amount: ab.balance,
            })
            .collect();

        let mut equity: Vec<BalanceSheetLine> = equity_balances
            .into_iter()
            .map(|ab| BalanceSheetLine {
                account_name: ab.account.name,
                amount: ab.balance,
            })
            .collect();

        // Net income (revenue - expenses) up to as_of, injected as an equity line so the sheet balances
        let revenue_balances =
            balances_by_category(db, &[AccountCategory::Revenue], None, as_of).await?;
        let expense_balances =
            balances_by_category(db, &[AccountCategory::Expense], None, as_of).await?;

        let total_revenue: Decimal = revenue_balances.iter().map(|ab| ab.balance).sum();
        let total_expense: Decimal = expense_balances.iter().map(|ab| ab.balance).sum();
        let current_period_net_income = total_revenue - total_expense;

        equity.push(BalanceSheetLine {
            account_name: "Net Income".to_string(),
            amount: current_period_net_income,
        });

        let total_assets: Decimal = assets.iter().map(|l| l.amount).sum();
        let total_liabilities: Decimal = liabilities.iter().map(|l| l.amount).sum();
        let total_equity: Decimal = equity.iter().map(|l| l.amount).sum();
        let total_liabilities_and_equity = total_liabilities + total_equity;

        Ok(BalanceSheet {
            as_of,
            assets,
            total_assets,
            liabilities,
            total_liabilities,
            equity,
            total_equity,
            total_liabilities_and_equity,
            current_period_net_income,
        })
    }
}
