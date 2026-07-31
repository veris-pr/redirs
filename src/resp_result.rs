use std::fmt;
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq)]
pub enum RESPError {
    FromUtf8,
    OutOfBounds(usize),
    WrongType,
    Unknown,
}

impl fmt::Display for RESPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RESPError::FromUtf8 => write!(f, "Failed to convert bytes to UTF-8 string"),
            RESPError::OutOfBounds(index) => write!(f, "Index {} is out of bounds", index),
            RESPError::WrongType => write!(f, "Unexpected type byte"),
            RESPError::Unknown => write!(f, "Unknown format for RESP string"),
        }
    }
}

impl From<FromUtf8Error> for RESPError {
    fn from(_: FromUtf8Error) -> Self {
        RESPError::FromUtf8
    }
}

pub type RESPResult<T> = Result<T, RESPError>;
