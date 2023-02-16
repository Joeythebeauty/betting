# betting
A Rust crate to manage twitch-style bets (aka [Parimutuel betting](https://en.wikipedia.org/wiki/Parimutuel_betting))

```rust
use bets::{Bets, BetError, Amount};

fn bet_demo() -> Result<(), BetError> {
    let bets = Bets::new("bets.db")?;
    // Create 3 accounts on server 1 with 100 starting coins
    bets.create_account("server 1", "Alice", 100)?;
    bets.create_account("server 1", "Bob", 100)?;
    bets.create_account("server 1", "Charlie", 100)?;
    // Create a bet with 2 outcomes
    let bet_id = bets.create(
        "server 1",
        "Who will win the Rocket League 1v1 ?",
        vec!["Alice", "Bob"],
    )?;
    // Alice bets on herself (option with id 0) with 10 coins
    bets.bet("server 1", "Alice", bet_id, 0, 10)?;
    // Bob bets on himself (option with id 1) with 40 coins
    bets.bet("server 1", "Bob", bet_id, 1, 40)?;
    // Charlie bets on Alice with half of his coins (50)
    bets.bet("server 1", "Charlie", bet_id, 0, Amount::FRACTION(0.5))?;
    // asserts that the money is gone from their accounts
    assert_eq!(bets.balance("server 1", "Alice")?, 90);
    assert_eq!(bets.balance("server 1", "Bob")?, 60);
    assert_eq!(bets.balance("server 1", "Charlie")?, 50);
    // lock the bet
    bets.lock_bet(bet_id)?;
    // ...
    // Rocket league 1v1 occurs
    // ...
    // Alice ended up winning ! we resolve the bet with option of id 0
    bets.close_bet(bet_id, 0)?;
    // The winning side gets 10 + 40 + 50 = 100 coins
    // split proportionally among the betters
    assert_eq!(bets.balance("server 1", "Alice"), 116);
    assert_eq!(bets.balance("server 1", "Charlie"), 184);
    assert_eq!(bets.balance("server 1", "Bob")?, 60);
    Ok(());
}

fn main() {
    if let Err(why) = bet_demo() {
        println!("{:?}", why);
    }
}
```