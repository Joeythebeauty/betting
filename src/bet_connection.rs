use rusqlite::Connection;
use crate::{BetError, Option, BetInfo};

pub(crate) trait BetConnection {
    fn options_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError>;

    fn option_status(
        &self, bet: u64, option: u64,
    ) -> Result<Option, BetError>;

    fn options_statuses(
        &self, bet: u64
    ) -> Result<Vec<Option>, BetError>;

    fn assert_bet_not_deleted(&self, bet: u64) -> Result<(), BetError>;

    fn bet_info(&self, bet: u64) -> Result<BetInfo, BetError>;

    fn balance(&self, server: u64, user: u64) -> Result<u64, BetError>;
}

impl BetConnection for Connection {
    fn options_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError> {
        Ok(self.prepare(
                "SELECT number 
                FROM Option
                WHERE bet = ?1",
            )
            .unwrap()
            .query_map([bet], |row| row.get::<usize, u64>(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn option_status(
        &self, bet: u64, option: u64,
    ) -> Result<Option, BetError> {
        let desc = self.prepare(
            "SELECT desc
            FROM Option
            WHERE bet = ?1 AND number = ?2",
            )
            .unwrap()
            .query_row([bet, option], |row| row.get::<usize, String>(0))?;
        let mut stmt = self
            .prepare(
                "SELECT user, amount
                FROM Wager
                WHERE bet = ?1 AND option = ?2",
            )
            .unwrap();
        let mut rows = stmt.query([bet, option])?;
        let mut wagers = Vec::new();
        while let Some(row) = rows.next()? {
            wagers.push((row.get::<usize, u64>(0)?, row.get::<usize, u64>(1)?));
        }
        Ok(Option {
            desc: desc,
            wagers: wagers,
        })
    }

    fn options_statuses(
        &self, bet: u64
    ) -> Result<Vec<Option>, BetError> {
        let options = self.options_of_bet(bet)?;
        options
            .into_iter()
            .map(|opt| self.option_status(bet, opt))
            .collect::<Result<Vec<_>, _>>()
    }

    fn assert_bet_not_deleted(&self, bet: u64) -> Result<(), BetError> {
        if self.prepare(
            "SELECT * 
            FROM ToDelete
            WHERE bet = ?1
            ",
        )
        .unwrap()
        .exists([bet])?
        {
            Err(BetError::NotFound)
        } else {
            Ok(())
        }
    }

    fn bet_info(&self, bet: u64) -> Result<BetInfo, BetError> {
        let (desc, server, is_open) = self.prepare(
            "SELECT desc, server, is_open 
            FROM Bet
            WHERE uuid = ?1
            ",
        )
        .unwrap()
        .query_row(
            [bet], 
            |row| 
            Ok((
                row.get::<usize, String>(0)?,
                row.get::<usize, u64>(1)?, 
                row.get::<usize, u32>(2)? != 0
            ))
        )?;
        Ok(BetInfo { desc, server, is_open })
    }

    fn balance(&self, server: u64, user: u64) -> Result<u64, BetError> {
        Ok(self.prepare(
                "SELECT balance 
                    FROM Account
                    WHERE server = ?1 AND user = ?2",
            )
            .unwrap()
            .query_row([server, user], |row| row.get::<usize, u64>(0))?)
    }
}