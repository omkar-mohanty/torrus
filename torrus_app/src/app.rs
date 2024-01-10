use std::error::Error;

pub trait AppFactory {
    type Err: Error;
}
