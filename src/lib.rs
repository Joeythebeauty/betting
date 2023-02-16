mod amount;
mod bets;
mod utils;
pub use amount::Amount;
pub use bets::{Bets, BetError};

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn bet_demo() {
        let bets = Bets::new("bets.db").unwrap();
        // Create 3 accounts on server 1 with 100 starting coins
        bets.create_account("server 1", "Alice", 100).unwrap();
        bets.create_account("server 1", "Bob", 100).unwrap();
        bets.create_account("server 1", "Charlie", 100).unwrap();
        // Create a bet with 2 outcomes
        let bet_id = bets.create(
            "server 1",
            "Who will win the Rocket League 1v1 ?",
            &vec!["Alice", "Bob"],
        ).unwrap();
        // Alice bets on herself (option with id 0) with 10 coins
        bets.bet("server 1", "Alice", bet_id, 0, 10).unwrap();
        // Bob bets on himself (option with id 1) with 40 coins
        bets.bet("server 1", "Bob", bet_id, 1, 40).unwrap();
        // Charlie bets on Alice with half of his coins (50)
        bets.bet("server 1", "Charlie", bet_id, 0, Amount::FRACTION(0.5)).unwrap();
        // asserts that the money is gone from their accounts
        assert_eq!(bets.balance("server 1", "Alice").unwrap(), 90);
        assert_eq!(bets.balance("server 1", "Bob").unwrap(), 60);
        assert_eq!(bets.balance("server 1", "Charlie").unwrap(), 50);
        // lock the bet
        bets.lock(bet_id).unwrap();
        // ...
        // Rocket league 1v1 occurs
        // ...
        // Alice ended up winning ! we resolve the bet with option of id 0
        bets.resolve(bet_id, 0).unwrap();
        // The winning side gets 10 + 40 + 50 = 100 coins
        // split proportionally among the betters
        assert_eq!(bets.balance("server 1", "Alice").unwrap(), 116);
        assert_eq!(bets.balance("server 1", "Charlie").unwrap(), 184);
        assert_eq!(bets.balance("server 1", "Bob").unwrap(), 60);
        Ok(());
    }
}
