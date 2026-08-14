use crate::resp::RESP;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ServerError {
    CommandInternalError(String),
    CommandSyntaxError(String),
    CommandNotAvailable(String),
    IncorrectData,
    StorageNotInitialised,
    HandshakeFailed(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::CommandInternalError(string) => {
                write!(f, "Internal error while processing {}.", string)
            }

            ServerError::CommandSyntaxError(string) => {
                write!(f, "Syntax error while processing {}.", string)
            }
            ServerError::IncorrectData => {
                write!(f, "Data received from stream is incorrect.")
            }
            ServerError::StorageNotInitialised => {
                write!(f, "Storage has not been initialised.")
            }
            ServerError::CommandNotAvailable(string) => {
                write!(f, "Command {} is not available.", string)
            }
            ServerError::HandshakeFailed(string) => {
                write!(f, "Handshake failed: {}.", string)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ServerValue {
    None,
    RESP(RESP),
    Binary(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum ServerMessage {
    Data(ServerValue),
    Error(ServerError),
}

pub type ServerResult = Result<ServerValue, ServerError>;
