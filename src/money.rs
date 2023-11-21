use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Money {
    pub amount: i64,
    pub currency: Currency,
}

impl Default for Money {
    fn default() -> Self {
        Money {
            amount: 0,
            currency: Currency::EUR,
        }
    }
}

impl Into<i64> for Money {
    fn into(self) -> i64 {
        self.amount
    }
}

impl Into<f32> for Money {
    fn into(self) -> f32 {
        self.amount as f32
    }
}


impl Display for Money {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02}{}", self.amount as f32 / self.currency.subdivision(), self.currency)
    }
}

impl Money {
    fn new(amount: i64, currency: &str) -> Self {
        Money {
            amount,
            currency: currency.into(),
        }
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currencies must match!");
        let rs = self.amount + rhs.amount;
        Money {
            amount: rs,
            currency: self.currency,
        }
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.amount = self.amount + rhs.amount
    }
}

impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currencies must match!");
        let rs = self.amount - rhs.amount;
        Money {
            amount: rs,
            currency: self.currency,
        }
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.amount = self.amount - rhs.amount
    }
}

impl<T: Sized + Into<i64>> Mul<T> for Money {
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        let rs = self.amount * rhs.into();
        Money {
            amount: rs,
            currency: self.currency,
        }
    }
}

impl<T: Sized + Into<i64>> MulAssign<T> for Money
    where T: Into<i64> {
    fn mul_assign(&mut self, rhs: T) {
        self.amount = self.amount * rhs.into()
    }
}

impl<T: Sized + Into<i64>> Div<T> for Money
    where T: Into<i64> {
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        let rs = self.amount / rhs.into();
        Money {
            amount: rs,
            currency: self.currency,
        }
    }
}

impl<T: Sized + Into<i64>> DivAssign<T> for Money where T: Into<i64> {
    fn div_assign(&mut self, rhs: T) {
        self.amount = self.amount / rhs.into()
    }
}

impl<T: Sized + Into<i64>> Rem<T> for Money where T: Into<i64> {
    type Output = Self;
    fn rem(self, rhs: T) -> Self::Output {
        let rs = self.amount % rhs.into();
        Money {
            amount: rs,
            currency: self.currency,
        }
    }
}

impl<T: Sized + Into<i64>> RemAssign<T> for Money where T: Into<i64> {
    fn rem_assign(&mut self, rhs: T) {
        self.amount = self.amount % rhs.into()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_money() {
        let m = Money::new(100_00, "EUR");
        assert_eq!(format!("{}", m), "100.00€");
        let m = Money::new(10, "EUR");
        assert_eq!(format!("{}", m), "0.10€");
    }

    #[test]
    fn test_math() {
        let m1 = Money::new(100_00, "EUR");
        let m2 = Money::new(13_00, "EUR");

        assert_eq!(m1 + m2, Money::new(113_00, "EUR"));
        assert_eq!(m1 - m2, Money::new(87_00, "EUR"));
        assert_eq!(m1 * 20, Money::new(2000_00, "EUR"));
        assert_eq!(m1 / 10, Money::new(10_00, "EUR"));
        assert_eq!(m1 % 13, Money::new(3, "EUR"));
    }
}