use hyper::{http::uri::InvalidUri, http::Error as HyperHttpError, Error as HyperError};
use serde_bencode::Error as BencodeError;
use std::{
    error::Error,
    fmt::{self, Debug},
};
use url::ParseError;

#[derive(Debug)]
pub enum ConnectionError {
    Custom(String),
    Other(String),
}

impl Error for ConnectionError {}

impl<'a> fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(msg) => f.write_str(msg),
            Self::Other(formatter) => f.write_str(&formatter),
        }
    }
}

impl From<HyperError> for ConnectionError {
    fn from(value: HyperError) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<BencodeError> for ConnectionError {
    fn from(value: BencodeError) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<InvalidUri> for ConnectionError {
    fn from(value: InvalidUri) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<HyperHttpError> for ConnectionError {
    fn from(value: HyperHttpError) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<ParseError> for ConnectionError {
    fn from(value: ParseError) -> Self {
        Self::Other(value.to_string())
    }
}
