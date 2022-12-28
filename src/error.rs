use std::error::Error;

#[macro_export]
macro_rules! err{
    ($err:expr) => {
       return Err(ToorrusError::new($err)) 
    };
}

#[derive(Debug)]
pub struct TorrusError(String);

impl Error for TorrusError {}

impl TorrusError {
    pub fn new(msg: &str) -> Self {
        Self(msg.to_string())
    }
}

impl std::fmt::Display for TorrusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<std::io::Error> for TorrusError {
    fn from(value: std::io::Error) -> Self {
        TorrusError::new(&value.to_string())
    }
}
