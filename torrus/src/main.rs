use torrus::{error::Result, init};

#[tokio::main]
async fn main() -> Result<()> {
    init().await?;
    println!("Hello, world!");
    Ok(())
}
