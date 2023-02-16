mod amount;
mod bets;
mod utils;
pub use amount::Amount;
pub use bets::Bets;

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn create_db() {
        match Bets::new("bets.db") {
            Ok(bets) => {
                if let Err(why) = bets.create_account("server", "Teo", 10) {
                    println!("1 {:?}", why);
                }
                if let Err(why) = bets.create_account("server", "Manu", 10) {
                    println!("2 {:?}", why);
                }
                if let Err(why) = bets.create_account("server", "Roux", 10) {
                    println!("3 {:?}", why);
                }
                if let Err(why) = bets.create_bet(
                    "server",
                    "bet1",
                    "Will roux go to sleep soon ?",
                    &vec!["opt1", "opt2"],
                    &vec!["oui", "non"],
                ) {
                    println!("4 {:?}", why);
                }
                if let Err(why) = bets.bet_on("server", "opt1", "Roux", Amount::FRACTION(0.5)) {
                    println!("5 {:?}", why);
                }
                if let Err(why) = bets.bet_on("server", "opt2", "Teo", Amount::FRACTION(0.3)) {
                    println!("6 {:?}", why);
                }
                if let Err(why) = bets.bet_on("server", "opt2", "Manu", Amount::FRACTION(0.4)) {
                    println!("7 {:?}", why);
                }
                if let Err(why) = bets.close_bet("server", "opt1") {
                    println!("8 {:?}", why);
                }
            }
            Err(why) => println!("0 {:?}", why),
        };
    }

    #[test]
    fn delete_on_start() {
        match Bets::new("bets.db") {
            Err(why) => println!("{:?}", why),
            _ => {}
        }
    }
}
