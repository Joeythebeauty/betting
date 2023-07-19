use rusqlite::Connection;
use crate::{BetError, Outcome, BetInfo};

pub(crate) trait BetConnection {
    fn outcomes_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError>;

    fn outcome_status(
        &self, bet: u64, outcome: u64,
    ) -> Result<Outcome, BetError>;

    fn outcomes_statuses(
        &self, bet: u64
    ) -> Result<Vec<Outcome>, BetError>;

    fn assert_bet_not_deleted(&self, bet: u64) -> Result<(), BetError>;

    fn bet_info(&self, bet: u64) -> Result<BetInfo, BetError>;

    fn balance(&self, server: u64, user: u64) -> Result<u64, BetError>;
}

impl BetConnection for Connection {
    fn outcomes_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError> {
        Ok(self.prepare(
                "SELECT number 
                FROM Outcome
                WHERE bet = ?1",
            )
            .unwrap()
            .query_map([bet], |row| row.get::<usize, u64>(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn outcome_status(
        &self, bet: u64, outcome: u64,
    ) -> Result<Outcome, BetError> {
        let desc = self.prepare(
            "SELECT desc
            FROM Outcome
            WHERE bet = ?1 AND number = ?2",
            )
            .unwrap()
            .query_row([bet, outcome], |row| row.get::<usize, String>(0))?;
        let mut stmt = self
            .prepare(
                "SELECT user, amount
                FROM Wager
                WHERE bet = ?1 AND outcome = ?2",
            )
            .unwrap();
        let mut rows = stmt.query([bet, outcome])?;
        let mut wagers = Vec::new();
        while let Some(row) = rows.next()? {
            wagers.push((row.get::<usize, u64>(0)?, row.get::<usize, u64>(1)?));
        }
        Ok(Outcome {
            desc: desc,
            wagers: wagers,
        })
    }

    fn outcomes_statuses(
        &self, bet: u64
    ) -> Result<Vec<Outcome>, BetError> {
        let outcomes = self.outcomes_of_bet(bet)?;
        outcomes
            .into_iter()
            .map(|opt| self.outcome_status(bet, opt))
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
        let (desc, server, author, is_open) = self.prepare(
            "SELECT desc, server, author, is_open 
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
                row.get::<usize, u64>(2)?, 
                row.get::<usize, u32>(3)? != 0
            ))
        )?;
        Ok(BetInfo { desc, server, author, is_open })
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