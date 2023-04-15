use std::{fmt::Display, str::FromStr};
use anyhow::Error;

#[derive(Clone, Copy)]
pub enum Amount {
    FLAT(u64),
    FRACTION(f32)
}

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Amount::FLAT(value) => {
                write!(f, "{}", value)
            },
            Amount::FRACTION(part) => {
                if *part == 1. {
                    write!(f, "All in")    
                } else {
                    write!(f, "{}%", part)
                }
            }
        }
    }
}

impl FromStr for Amount {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.ends_with("%") {
            Ok(Amount::FRACTION(s.trim_end_matches("%").parse::<f32>()?/100.))
        } else {
            Ok(Amount::FLAT(s.parse()?))
        }
    }
}

impl From<u64> for Amount {
    fn from(val: u64) -> Self {
        Amount::FLAT(val)
    }
}

impl From<f32> for Amount {
    fn from(fract: f32) -> Self {
        Amount::FRACTION(fract)
    }
}