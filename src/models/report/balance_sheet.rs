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
        let all_balances = balances_by_category(
            db,
            &[
                AccountCategory::Asset,
                AccountCategory::Liability,
                AccountCategory::Equity,
                AccountCategory::Revenue,
                AccountCategory::Expense,
            ],
            None,
            as_of,
        )
        .await?;

        let mut assets = Vec::new();
        let mut liabilities = Vec::new();
        let mut equity: Vec<BalanceSheetLine> = Vec::new();
        let mut total_revenue = Decimal::ZERO;
        let mut total_expense = Decimal::ZERO;

        for ab in all_balances {
            let line = BalanceSheetLine {
                account_name: ab.account.name,
                amount: ab.balance,
            };
            match ab.account.category {
                AccountCategory::Asset => assets.push(line),
                AccountCategory::Liability => liabilities.push(line),
                AccountCategory::Equity => equity.push(line),
                AccountCategory::Revenue => total_revenue += ab.balance,
                AccountCategory::Expense => total_expense += ab.balance,
            }
        }

        let current_period_net_income = total_revenue - total_expense;

        // Inject net income as an equity line so the sheet balances
        if !equity.iter().any(|l| l.account_name == "Net Income") {
            equity.push(BalanceSheetLine {
                account_name: "Net Income".to_string(),
                amount: current_period_net_income,
            });
        }

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
