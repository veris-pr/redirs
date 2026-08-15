use std::fmt;
use std::num;
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq)]
pub enum RESPError {
    FromUtf8,
    OutOfBounds(usize),
    WrongType,
    Unknown,
    IncorrectLength(RESPLength),
    ParseInt,
}

impl fmt::Display for RESPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RESPError::FromUtf8 => write!(f, "Failed to convert bytes to UTF-8 string"),
            RESPError::OutOfBounds(index) => write!(f, "Index {} is out of bounds", index),
            RESPError::WrongType => write!(f, "Unexpected type byte"),
            RESPError::Unknown => write!(f, "Unknown format for RESP string"),
            RESPError::IncorrectLength(length) => write!(f, "Incorrect length {}", length),
            RESPError::ParseInt => write!(f, "Cannot parse string into integer"),
        }
    }
}

impl From<FromUtf8Error> for RESPError {
    fn from(_: FromUtf8Error) -> Self {
        RESPError::FromUtf8
    }
}

impl From<num::ParseIntError> for RESPError {
    fn from(_: num::ParseIntError) -> Self {
        RESPError::ParseInt
    }
}

pub type RESPResult<T> = Result<T, RESPError>;

pub type RESPLength = i32;
