use rusqlite::{Transaction, params};
use crate::{AccountUpdate, BetError};

pub(crate) trait BetTransaction {
    fn change_balance(&self, server: u64, user: u64, amount: i64) -> Result<AccountUpdate, BetError>;
}

impl BetTransaction for Transaction<'_> {
    fn change_balance(&self, server: u64, user: u64, amount: i64) -> Result<AccountUpdate, BetError> {
        let balance = self.query_row(
            "UPDATE Account
            SET balance = balance + ?1
            WHERE server = ?2 AND user = ?3
            RETURNING balance",
            params![amount, server, user],
            |row | row.get::<usize, u64>(0)
        )?;
        Ok(AccountUpdate {
            server, user, diff: amount, balance,
        })
    }
}