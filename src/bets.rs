use crate::{utils, amount::Amount, BetError, AccountUpdate, Bet, AccountStatus, bet_connection::BetConnection, bet_transaction::BetTransaction};
use rusqlite::{Connection, Result, Transaction, params};
use std::collections::HashMap;
use itertools::izip;

#[derive(Debug, Clone)]
pub struct Bets {
    db_path: String,
}

impl Bets {
    pub fn new(db_path: &str) -> Result<Self, BetError> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Account (
                server INTEGER,
                user INTEGER,
                balance INTEGER NOT NULL,
                PRIMARY KEY(server, user)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Bet (
                uuid INTEGER PRIMARY KEY,
                server INTEGER,
                is_open INTEGER NOT NULL,
                desc TEXT
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Outcome (
                bet INTEGER,
                number INTEGER,
                desc TEXT,
                PRIMARY KEY(bet, number)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Wager (
                bet INTEGER,
                outcome INTEGER,
                server INTEGER,
                user INTEGER,
                amount INTEGER NOT NULL,
                FOREIGN KEY(bet, outcome) REFERENCES Outcome(bet, number) ON DELETE CASCADE,
                FOREIGN KEY(server, user) REFERENCES Account(server, user) ON DELETE CASCADE,
                PRIMARY KEY(user, bet)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ToDelete (
                bet INTEGER PRIMARY KEY REFERENCES Bet(uuid) ON DELETE CASCADE
            )",
            [],
        )?;
        conn.execute(
            "DELETE FROM Bet
            WHERE EXISTS (
                SELECT bet 
                FROM ToDelete 
                WHERE ToDelete.bet = Bet.uuid
            )",
            [],
        )?;
        Ok(Bets {
            db_path: db_path.to_string(),
        })
    }

    pub fn create_account(&self, server: u64, user: u64, amount: u64) -> Result<(), BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT 
            INTO Account (server, user, balance) 
            VALUES (?1, ?2, ?3)",
            [server, user, amount],
        )?;
        Ok(())
    }

    pub fn reset(&self, server: u64, amount: u64) -> Result<(), BetError> {
        let mut conn = Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE
            FROM Bet
            WHERE server = ?1",
            [server],
        )?;
        tx.execute(
            "UPDATE Account
            SET balance = ?1
            WHERE server = ?2",
            [amount, server],
        )?;
        Ok(tx.commit()?)
    }

    pub fn global_income(&self, income: u64) -> Result<(), BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE Account
            SET balance = balance + ?1", 
            [income]
        )?;
        Ok(())
    }

    pub fn income(&self, server: u64, income: u64) -> Result<Vec<AccountUpdate>, BetError> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "UPDATE Account
            SET balance = balance + ?1
            WHERE server = ?2
            RETURNING server, user, balance"
        ).unwrap();
        let mut account_updates = Vec::new();
        let mut rows = stmt.query([server, income])?;
        while let Some(row) = rows.next()? {
            account_updates.push(AccountUpdate {
                server: row.get::<usize, u64>(0)?,
                user: row.get::<usize, u64>(1)?,
                balance: row.get::<usize, u64>(2)?,
                diff: income as i64,
            });
        }
        Ok(account_updates)
    }

    pub fn create_bet<S1, S2>(
        &self,
        bet_uuid: u64,
        server: u64,
        desc: S1,
        outcomes: &[S2],
    ) -> Result<(), BetError>
    where S1: ToString, S2: ToString {
        let desc = desc.to_string();
        let mut conn = Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT 
            INTO Bet (uuid, server, is_open, desc) 
            VALUES (?1, ?2, ?3, ?4)",
            params![bet_uuid, server, 1, desc],
        )?;
        for (i, opt) in outcomes.into_iter().enumerate() {
            tx.execute(
                "INSERT 
                INTO Outcome (bet, number, desc) 
                VALUES (?1, ?2, ?3)",
                params![bet_uuid, i, opt.to_string()],
            )?;
        }
        Ok(tx.commit()?)
    }

    pub fn outcomes_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.outcomes_of_bet(bet)
    }

    pub fn bet_on<A>(
        &self,
        bet: u64,
        outcome: usize,
        user: u64,
        amount: A,
    ) -> Result<(AccountUpdate, Bet), BetError>
    where A: Into<Amount> {
        let amount: Amount = amount.into();
        let mut conn = Connection::open(&self.db_path)?;
        // check if the bet is open
        let bet_info = conn.bet_info(bet)?;
        if !bet_info.is_open {
            return Err(BetError::BetLocked);
        }
        conn.assert_bet_not_deleted(bet)?;
        // compute the amount to bet
        let balance = conn.balance(bet_info.server, user)?;
        let amount = match amount {
            Amount::FLAT(value) => {
                if value > balance {
                    return Err(BetError::NotEnoughMoney);
                }
                value
            },
            Amount::FRACTION(part) => {
                assert!(0. <= part && part <= 1.);
                let value = f32::ceil(balance as f32 * part) as u64;
                if value == 0 {
                    return Err(BetError::NotEnoughMoney);
                }
                value
            }
        };
        // bet
        let tx = conn.transaction()?;
        let acc_update = tx.change_balance(bet_info.server, user, -(amount as i64))?;
        tx.execute(
            "INSERT or ignore
            INTO Wager (bet, outcome, server, user, amount)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![bet, outcome, bet_info.server, user, 0],
        )?;
        tx.execute(
            "UPDATE Wager
            SET amount = amount + ?1
            WHERE bet = ?2 AND outcome = ?3 AND user = ?4
            ",
            params![amount, bet, outcome, user],
        )?;
        tx.commit()?;
        Ok((
            acc_update,
            Bet {
                bet: bet.clone(),
                desc: bet_info.desc,
                outcomes: conn.outcomes_statuses(bet)?,
                is_open: bet_info.is_open,
                server: bet_info.server
            },
        ))
    }

    pub fn lock_bet(&self, bet: u64) -> Result<(), BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE Bet
            SET is_open = 0
            WHERE uuid = ?1",
            [bet],
        )?;
        Ok(())
    }

    fn delete_bet(
        tx: &Transaction,
        bet: u64,
    ) -> Result<(), BetError> {
        tx.execute(
            "INSERT 
            INTO ToDelete (bet)
            VALUES (?1)",
            [bet],
        )?;
        Ok(())
    }

    pub fn abort_bet(&self, bet: u64) -> Result<Vec<AccountUpdate>, BetError> {
        let bet = bet;
        let mut conn = Connection::open(&self.db_path)?;
        conn.assert_bet_not_deleted(bet)?;
        let bet_info = conn.bet_info(bet)?;
        let outcomes = conn.outcomes_statuses(bet)?;
        let wagers: Vec<(u64, u64)> = outcomes
            .iter()
            .flat_map(|outcome_status| outcome_status.wagers.clone())
            .collect();
        let mut account_updates = Vec::new();
        let tx = conn.transaction()?;
        for (user, amount) in wagers {
            account_updates.push(tx.change_balance(bet_info.server, user, amount as i64)?);
        }
        // delete the bet
        Bets::delete_bet(
            &tx, bet
        )?;
        tx.commit()?;
        Ok(account_updates)
    }

    pub fn resolve(
        &self,
        bet: u64,
        winning_outcome: usize,
    ) -> Result<Vec<AccountUpdate>, BetError> {
        let mut conn = Connection::open(&self.db_path)?;
        let bet_info = conn.bet_info(bet)?;
        // retrieve the total of the bet and the winning parts
        conn.assert_bet_not_deleted(bet)?;
        let outcomes_statuses = conn.outcomes_statuses(bet)?;
        let mut winners: Vec<u64> = Vec::new();
        let mut wins: Vec<u64> = Vec::new();
        let mut total = 0;
        for (i, outcome_status) in outcomes_statuses.iter().enumerate() {
            let outcome_sum = outcome_status
                .wagers
                .iter()
                .fold(0, |init, wager| init + wager.1);
            total += outcome_sum;
            if i == winning_outcome {
                for (winner, win) in &outcome_status.wagers {
                    winners.push(*winner);
                    wins.push(*win);
                }
            }
        }
        // compute the gains for each winners
        let gains = utils::lrm(total, &wins);
        // update the accounts
        let mut account_updates = Vec::new();
        let tx = conn.transaction()?;
        for (user, gain) in izip!(winners, gains) {
            account_updates.push(tx.change_balance(bet_info.server, user, gain as i64)?);
        }
        // delete the bet
        Bets::delete_bet(&tx, bet)?;
        tx.commit()?;
        Ok(account_updates)
    }

    pub fn balance(&self, server: u64, user: u64) -> Result<u64, BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.balance(server, user)
    }

    pub fn accounts(&self, server: &str) -> Result<Vec<AccountStatus>, BetError> {
        let conn = Connection::open(&self.db_path)?;
        // Map <user, balance>
        let mut accounts = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT user, balance 
                    FROM Account
                    WHERE server = ?1
                    ",
            )
            .unwrap();
        let mut rows = stmt.query(&[server])?;
        while let Some(row) = rows.next()? {
            accounts.insert(row.get::<usize, String>(0)?, row.get::<usize, u32>(1)?);
        }
        // Map <user, total wagered>
        let mut stmt = conn
            .prepare(
                "SELECT user, amount 
                    FROM Wager
                    WHERE server = ?1",
            )
            .unwrap();
        let mut rows = stmt.query(&[server])?;
        let mut wagers = HashMap::new();
        while let Some(row) = rows.next()? {
            let user = row.get::<usize, String>(0)?;
            let amount = match wagers.get(&user) {
                Some(amount) => *amount,
                None => 0,
            };
            wagers.insert(user, amount + row.get::<usize, u32>(1)?);
        }
        // return the account statuses
        Ok(accounts
            .into_iter()
            .map(|(user, balance)| AccountStatus {
                user: user.clone(),
                balance: balance,
                in_bet: *wagers.get(&user).unwrap_or(&0),
            })
            .collect())
    }
}