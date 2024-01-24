use std::fmt::{Display, Formatter};
pub use anyhow::{Result, anyhow, Context};

#[derive(Debug, PartialEq)]
pub enum SplitterError {
    InvalidTargetFormat,
    LogicError,
    InvalidName,
    MemberNotFound,
    GroupNotFound,
    LogEntryNotFound,
}

impl std::error::Error for SplitterError {}

impl Display for SplitterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LogicError => { write!(f, "Something does not add up")? }
            Self::InvalidTargetFormat => { write!(f, "Invalid Format for the from or to directives")? }
            Self::InvalidName => { write!(f, "Name Invalid: Must match the following Regex: `^[a-zA-Z0-9][a-zA-Z0-9_\\-()]*$`")? }
            Self::MemberNotFound => { write!(f, "Member not found")? }
            Self::GroupNotFound => { write!(f, "Group not found")? }
            Self::LogEntryNotFound => { write!(f, "Log Entry not found")? }
        }
        Ok(())
    }
}