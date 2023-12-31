use torrus::{error::Result, init};

fn main() -> Result<()> {
    init()?;
    println!("Hello, world!");
    Ok(())
}
