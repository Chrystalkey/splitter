use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Currency {
    EUR,
    USD,
    JPY,
    GBP,
}

impl Currency {
    fn subdivision(&self) -> f32 {
        match self {
            Self::EUR |
            Self::USD |
            Self::GBP |
            Self::JPY => 100.,
        }
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let sign = match self {
            Currency::EUR => "€",
            Currency::USD => "$",
            Currency::GBP => "£",
            Currency::JPY => "¥"
        };
        write!(f, "{}", sign)
    }
}

impl From<&str> for Currency {
    fn from(value: &str) -> Self {
        match value {
            "EUR" => Self::EUR,
            "USD" => Self::USD,
            "JPY" => Self::JPY,
            "GBP" => Self::GBP,
            _ => { panic!("Currency \"{}\" not supported!", value) }
        }
    }
}
