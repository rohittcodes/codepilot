use anyhow::Result;
use codepilot::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Create and run the CLI application
    let mut app = App::new()?;
    app.run().await?;

    Ok(())
}