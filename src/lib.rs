mod amount;
mod db_structs;
mod bet_connection;
mod bet_transaction;
mod bets;
mod utils;
pub use amount::Amount;
pub use bets::Bets;
pub use db_structs::*;

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn bet_demo() -> Result<(), BetError> {
        // variables for readability
        let server_id = 1;
        let bet_id = 1;
        let (alice, bob, charlie) = (0, 1, 2);
        // Create the database
        let bets = Bets::new("bets.db")?;
        // Create 3 accounts on server 1 with 100 starting coins
        bets.create_account(server_id, alice, 100)?;
        bets.create_account(server_id, bob, 100)?;
        bets.create_account(server_id, charlie, 100)?;
        // Create a bet with 2 outcomes
        bets.create_bet(
            bet_id, server_id,
            "Who will win the Rocket League 1v1 ?",
            &vec!["Alice", "Bob"],
        )?;
        // Alice bets on herself (outcome with id 0) with 10 coins
        bets.bet_on(bet_id, 0, alice, 10)?;
        // Bob bets on himself (outcome with id 1) with 40 coins
        bets.bet_on(bet_id, 1, bob, 40)?;
        // Charlie bets on Alice with half of his coins (50)
        bets.bet_on(bet_id, 0, charlie, 0.5)?;
        // asserts that the money is gone from their accounts
        assert_eq!(bets.balance(server_id, alice)?, 90);
        assert_eq!(bets.balance(server_id, bob)?, 60);
        assert_eq!(bets.balance(server_id, charlie)?, 50);
        // lock the bet
        bets.lock_bet(bet_id)?;
        // ...
        // Rocket league 1v1 occurs
        // ...
        // Alice ended up winning ! we resolve the bet with outcome of id 0
        bets.resolve(bet_id, 0)?;
        // The winning side gets 10 + 40 + 50 = 100 coins
        // split proportionally among the betters
        // Alice had bet 10 out of 60 of this outcome, she wins 100*(10/60) = 16.6 rounded to 17
        assert_eq!(bets.balance(server_id, alice)?, 107);
        // Bob doesn't win anything since he bet on the wrong outcome
        assert_eq!(bets.balance(server_id, bob)?, 60);
        // Charlie had bet 50 out of 60 of this outcome, he wins 100*(50/60) = 83.3 rounded to 83
        assert_eq!(bets.balance(server_id, charlie)?, 133);
        Ok(())
    }
}
