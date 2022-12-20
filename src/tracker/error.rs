use hyper::{
    http::{uri::InvalidUri, Error as HttpError},
    Error as HyperError,
};
use serde_bencode::Error as BencodeError;
use std::error::Error;
use url::ParseError as UrlError;

#[derive(Debug)]
pub enum TrackerError {
    BencodeError(BencodeError),
    HyperError(HyperError),
    HttpError(HttpError),
    UriError(InvalidUri),
    UrlError(UrlError),
}

impl Error for TrackerError {}

impl std::fmt::Display for TrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BencodeError(err) => err.fmt(f),
            Self::HyperError(err) => err.fmt(f),
            Self::UrlError(err) => err.fmt(f),
            Self::UriError(err) => err.fmt(f),
            Self::HttpError(err) => err.fmt(f),
        }
    }
}

impl From<HyperError> for TrackerError {
    fn from(err: HyperError) -> Self {
        Self::HyperError(err)
    }
}

impl From<BencodeError> for TrackerError {
    fn from(err: BencodeError) -> Self {
        Self::BencodeError(err)
    }
}

impl From<UrlError> for TrackerError {
    fn from(err: UrlError) -> Self {
        Self::UrlError(err)
    }
}

impl From<InvalidUri> for TrackerError {
    fn from(err: InvalidUri) -> Self {
        Self::UriError(err)
    }
}

impl From<HttpError> for TrackerError {
    fn from(err: HttpError) -> Self {
        Self::HttpError(err)
    }
}
