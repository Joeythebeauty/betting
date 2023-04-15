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
    fn bet_demo() {
        let bets = Bets::new("bets.db").unwrap();
        // Create 3 accounts on server 1 with 100 starting coins
        bets.create_account(1u64, 0u64, 100u64).unwrap();
        bets.create_account(1u64, 1u64, 100u64).unwrap();
        bets.create_account(1u64, 2u64, 100u64).unwrap();
        // Create a bet with 2 outcomes
        bets.create_bet(
            1u64, 1u64,
            "Who will win the Rocket League 1v1 ?",
            &vec!["Alice", "Bob"],
        ).unwrap();
        // Alice bets on herself (option with id 0) with 10 coins
        bets.bet_on(1u64, 0u64, 0u64, 10).unwrap();
        // Bob bets on himself (option with id 1) with 40 coins
        bets.bet_on(1u64, 1u64, 1u64, 40).unwrap();
        // Charlie bets on Alice with half of his coins (50)
        bets.bet_on(1u64, 0u64, 2u64, 0.5).unwrap();
        // asserts that the money is gone from their accounts
        assert_eq!(bets.balance(1u64, 0u64).unwrap(), 90);
        assert_eq!(bets.balance(1u64, 1u64).unwrap(), 60);
        assert_eq!(bets.balance(1u64, 2u64).unwrap(), 50);
        // lock the bet
        bets.lock_bet(1u64).unwrap();
        // ...
        // Rocket league 1v1 occurs
        // ...
        // Alice ended up winning ! we resolve the bet with option of id 0
        bets.resolve(1u64, 0u64).unwrap();
        // The winning side gets 10 + 40 + 50 = 100 coins
        // split proportionally among the betters
        // Alice had bet 10 out of 60 of this option, she wins 100*(10/60) = 16.6 rounded to 17
        assert_eq!(bets.balance(1u64, 0u64).unwrap(), 107);
        // Bob doesn't win anything since he bet on the wrong option
        assert_eq!(bets.balance(1u64, 1u64).unwrap(), 60);
        // Charlie had bet 50 out of 60 of this option, he wins 100*(50/60) = 83.3 rounded to 83
        assert_eq!(bets.balance(1u64, 2u64).unwrap(), 133);
    }
}
