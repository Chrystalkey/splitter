use std::num::ParseFloatError;

#[derive(Debug)]
pub enum TargetParsingError {
    InvalidNumberFormat(String),
    InvalidFormat(String),
    InvalidSemantic(String),
}

impl From<ParseFloatError> for TargetParsingError {
    fn from(value: ParseFloatError) -> Self {
        return Self::InvalidNumberFormat(value.to_string());
    }
}

#[derive(Debug)]
pub enum InternalSplitterError {
    NoGroup(String),
    TargetParsingError(String),
}

impl From<TargetParsingError> for InternalSplitterError {
    fn from(value: TargetParsingError) -> Self {
        return InternalSplitterError::TargetParsingError(
            format!("TargetParsingError: {:?}", value)
        );
    }
}