
#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub server: u64,
    pub user: u64,
    pub diff: i64,
    pub balance: u64,
}

pub(crate) struct BetInfo {
    pub desc: String,
    pub server: u64,
    pub is_open: bool
}

pub struct Bet {
    pub bet: u64,
    pub server: u64,
    pub desc: String,
    pub outcomes: Vec<Outcome>,
    pub is_open: bool
}

pub struct Outcome {
    pub desc: String,
    // [(user, amount), ]
    pub wagers: Vec<(u64, u64)>,
}

pub struct AccountStatus {
    pub user: String,
    pub balance: u32,
    pub in_bet: u32,
}

#[derive(Debug)]
pub enum BetError {
    MultiOpt(Vec<String>),
    NotFound,
    NotEnoughMoney,
    BetLocked,
    AlreadyExists,
    InternalError(rusqlite::Error),
}

impl From<rusqlite::Error> for BetError {
    fn from(err: rusqlite::Error) -> Self {
        // the only error we want to separate is the unique constraint violation
        if let rusqlite::Error::SqliteFailure(sqlerr, _) = err {
            if sqlerr.extended_code == 1555 {
                return BetError::AlreadyExists;
            }
        } else if let rusqlite::Error::QueryReturnedNoRows = err {
            return BetError::NotFound;
        }
        BetError::InternalError(err)
    }
}