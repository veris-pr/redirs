use std::fmt;

#[derive(Debug, PartialEq)]
pub enum StorageError {
    CommandSyntaxError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::CommandSyntaxError(cmd) => write!(f, "Syntax error in command: {}", cmd),
        }
    }
}

pub type StorageResult<T> = Result<T, StorageError>;
