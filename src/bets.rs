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
            "CREATE TABLE IF NOT EXISTS Option (
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
                option INTEGER,
                server INTEGER,
                user INTEGER,
                amount INTEGER NOT NULL,
                FOREIGN KEY(bet, option) REFERENCES Option(bet, number) ON DELETE CASCADE,
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

    pub fn create_account<U1, U2, U3>(&self, server: U1, user: U2, amount: U3) -> Result<(), BetError> 
    where U1: Into<u64>, U2: Into<u64>, U3: Into<u64> {
        let conn = Connection::open(&self.db_path)?;
        let user: u64 = user.into();
        conn.execute(
            "INSERT 
            INTO Account (server, user, balance) 
            VALUES (?1, ?2, ?3)",
            [server.into(), user, amount.into()],
        )?;
        Ok(())
    }

    pub fn reset<U1, U2>(&self, server: U1, amount: U2) -> Result<(), BetError>
    where U1: Into<u64>, U2: Into<u64> {
        let server: u64 = server.into();
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
            [amount.into(), server],
        )?;
        Ok(tx.commit()?)
    }

    pub fn income<U1, U2>(&self, server: U1, income: U2) -> Result<Vec<AccountUpdate>, BetError> 
    where U1: Into<u64>, U2: Into<u64> {
        let server: u64 = server.into();
        let income: u64 = income.into();
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

    pub fn create_bet<U1, U2, S1, S2>(
        &self,
        bet_uuid: U1,
        server: U2,
        desc: S1,
        options: &[S2],
    ) -> Result<(), BetError>
    where U1: Into<u64>, U2: Into<u64>, S1: ToString, S2: ToString {
        let bet_uuid = bet_uuid.into();
        let server = server.into();
        let desc = desc.to_string();
        let mut conn = Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT 
            INTO Bet (uuid, server, is_open, desc) 
            VALUES (?1, ?2, ?3, ?4)",
            params![bet_uuid, server, 1, desc],
        )?;
        for (i, opt) in options.into_iter().enumerate() {
            tx.execute(
                "INSERT 
                INTO Option (bet, number, desc) 
                VALUES (?1, ?2, ?3)",
                params![bet_uuid, i, opt.to_string()],
            )?;
        }
        Ok(tx.commit()?)
    }

    pub fn options_of_bet(&self, bet: u64) -> Result<Vec<u64>, BetError> {
        let conn = Connection::open(&self.db_path)?;
        conn.options_of_bet(bet)
    }

    pub fn bet_on<U1, U2, U3, A>(
        &self,
        bet: U1,
        option: U2,
        user: U3,
        amount: A,
    ) -> Result<(AccountUpdate, Bet), BetError>
    where U1: Into<u64>, U2: Into<u64>, U3: Into<u64>, A: Into<Amount> {
        let bet: u64 = bet.into();
        let option: u64 = option.into();
        let user: u64 = user.into();
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
            INTO Wager (bet, option, server, user, amount)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            [bet, option, bet_info.server, user, 0],
        )?;
        tx.execute(
            "UPDATE Wager
            SET amount = amount + ?1
            WHERE bet = ?2 AND option = ?3 AND user = ?4
            ",
            [amount, bet, option, user],
        )?;
        tx.commit()?;
        Ok((
            acc_update,
            Bet {
                bet: bet.clone(),
                desc: bet_info.desc,
                options: conn.options_statuses(bet)?,
                is_open: bet_info.is_open,
                server: bet_info.server
            },
        ))
    }

    pub fn lock_bet<U: Into<u64>>(&self, bet: U) -> Result<(), BetError> {
        let bet = bet.into();
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE Bet
            SET is_open = 0
            WHERE uuid = ?1",
            [bet],
        )?;
        Ok(())
    }

    fn delete_bet<U: Into<u64>>(
        tx: &Transaction,
        bet: U,
    ) -> Result<(), BetError> {
        tx.execute(
            "INSERT 
            INTO ToDelete (bet)
            VALUES (?1)",
            [bet.into()],
        )?;
        Ok(())
    }

    pub fn abort_bet<U: Into<u64>>(&self, bet: U) -> Result<Vec<AccountUpdate>, BetError> {
        let bet = bet.into();
        let mut conn = Connection::open(&self.db_path)?;
        conn.assert_bet_not_deleted(bet)?;
        let bet_info = conn.bet_info(bet)?;
        let options = conn.options_statuses(bet)?;
        let wagers: Vec<(u64, u64)> = options
            .iter()
            .flat_map(|option_status| option_status.wagers.clone())
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

    pub fn resolve<U1, U2>(
        &self,
        bet: U1,
        winning_option: U2,
    ) -> Result<Vec<AccountUpdate>, BetError>
    where U1: Into<u64>, U2: Into<u64> {
        let bet = bet.into();
        let winning_option = winning_option.into() as usize; 
        let mut conn = Connection::open(&self.db_path)?;
        let bet_info = conn.bet_info(bet)?;
        // retrieve the total of the bet and the winning parts
        conn.assert_bet_not_deleted(bet)?;
        let options_statuses = conn.options_statuses(bet)?;
        let mut winners: Vec<u64> = Vec::new();
        let mut wins: Vec<u64> = Vec::new();
        let mut total = 0;
        for (i, option_status) in options_statuses.iter().enumerate() {
            let option_sum = option_status
                .wagers
                .iter()
                .fold(0, |init, wager| init + wager.1);
            total += option_sum;
            if i == winning_option {
                for (winner, win) in &option_status.wagers {
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

    pub fn balance<U1, U2>(&self, server: U1, user: U2) -> Result<u64, BetError>
    where U1: Into<u64>, U2: Into<u64> {
        let conn = Connection::open(&self.db_path)?;
        conn.balance(server.into(), user.into())
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