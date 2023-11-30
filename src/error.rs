use std::io::Error;
use std::num::ParseFloatError;

#[derive(Debug, PartialEq)]
pub enum InternalSplitterError {
    InvalidNumberFormat(String),
    InvalidTargetFormat(String),
    InvalidSemantic(String),
    InvalidName(String),
    DatabaseReadError(String),
    FileError(String),
    GroupNotFound,
    MemberNotFound(String),
    LogEntryNotFound,
}
// todo: impl Display for InternalSplitterError

impl From<ParseFloatError> for InternalSplitterError {
    fn from(value: ParseFloatError) -> Self {
        Self::InvalidNumberFormat(value.to_string())
    }
}

impl From<Error> for InternalSplitterError {
    fn from(value: Error) -> Self {
        Self::FileError(
            format!("{:?}", value)
        )
    }
}

impl From<serde_yaml::Error> for InternalSplitterError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::DatabaseReadError(
            format!("{:?}", value)
        )
    }
}