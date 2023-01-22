use super::connection::error::ConnectionError;
use std::error::Error;

#[derive(Debug)]
pub(crate) enum TrackerError {
    Other(String),
}

impl Error for TrackerError {}

impl std::fmt::Display for TrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(msg) => f.write_str(&msg),
        }
    }
}

impl From<ConnectionError> for TrackerError {
    fn from(value: ConnectionError) -> Self {
        Self::Other(value.to_string())
    }
}
