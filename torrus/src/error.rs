use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct TorrErr(String);

impl Display for TorrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)?;
        Ok(())
    }
}

impl std::error::Error for TorrErr {}

impl From<std::io::Error> for TorrErr {
    fn from(value: std::io::Error) -> Self {
        TorrErr(value.to_string())
    }
}
