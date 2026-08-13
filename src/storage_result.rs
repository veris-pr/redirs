use std::fmt;

#[derive(Debug, PartialEq)]
pub enum StorageError {
    CommandNotAvailable(String),
    CommandSyntaxError(String),
    CommandInternalError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::CommandNotAvailable(cmd) => write!(f, "Command not available: {}", cmd),
            StorageError::CommandSyntaxError(cmd) => write!(f, "Syntax error in command: {}", cmd),
            StorageError::CommandInternalError(cmd) => {
                write!(f, "Internal error in command: {}", cmd)
            }
        }
    }
}

pub type StorageResult<T> = Result<T, StorageError>;
