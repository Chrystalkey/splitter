use std::io::Error;
use std::num::ParseFloatError;

#[derive(Debug)]
pub enum InternalSplitterError {
    InvalidNumberFormat(String),
    InvalidFormat(String),
    InvalidSemantic(String),
    InvalidName(String),
    FileError(String),
}

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
        Self::InvalidFormat(
            format!("{:?}", value)
        )
    }
}