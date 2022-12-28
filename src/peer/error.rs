use std::fmt::Display;

#[derive(Debug)]
pub enum PeerError {
    Custom(String),
}

impl std::error::Error for PeerError {}

impl Display for PeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerError::Custom(msg) => f.write_str(&msg),
        }
    }
}

impl From<std::io::Error> for PeerError {
    fn from(value: std::io::Error) -> Self {
        Self::Custom(value.to_string())
    }
}
