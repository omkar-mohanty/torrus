use hyper::{http::uri::InvalidUri, http::Error as HyperHttpError, Error as HyperError};
use serde_bencode::Error as BencodeError;
use std::error::Error;
use url::ParseError;

#[derive(Debug)]
pub struct TorrusError(String);

type Result<'a, T> = std::result::Result<T, &'a str>;

impl Error for TorrusError {}

impl TorrusError {
    pub fn new(msg: &str) -> Self {
        Self(msg.to_string())
    }

    pub fn from_error<T: Error>(error: T) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for TorrusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'a, T> From<Result<'a, T>> for TorrusError {
    fn from(value: Result<'a, T>) -> Self {
        match value {
            Ok(val) => Ok(val).into(),
            Err(err) => TorrusError(err.to_string()),
        }
    }
}

impl From<std::io::Error> for TorrusError {
    fn from(value: std::io::Error) -> Self {
        TorrusError::new(&value.to_string())
    }
}

impl From<HyperError> for TorrusError {
    fn from(value: HyperError) -> Self {
        Self::new(&value.to_string())
    }
}

impl From<BencodeError> for TorrusError {
    fn from(value: BencodeError) -> Self {
        Self::new(&value.to_string())
    }
}

impl From<InvalidUri> for TorrusError {
    fn from(value: InvalidUri) -> Self {
        Self::new(&value.to_string())
    }
}

impl From<HyperHttpError> for TorrusError {
    fn from(value: HyperHttpError) -> Self {
        Self::new(&value.to_string())
    }
}

impl From<ParseError> for TorrusError {
    fn from(value: ParseError) -> Self {
        Self::new(&value.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for TorrusError {
    fn from(value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::new(&value.to_string())
    }
}

unsafe impl Send for TorrusError {}

#[cfg(test)]
mod tests {}
