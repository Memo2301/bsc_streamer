use anyhow::Result;
use bsc_streamer::{display::formatter::SwapFormatter, StreamerBuilder};
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (suppress ethers WebSocket errors)
    tracing_subscriber::fmt()
        .with_env_filter("bsc_streamer=info,ethers=warn,ethers_providers::rpc::transports::ws=off")
        .init();

    // Load environment variables
    dotenv().ok();

    let wss_url = env::var("BSC_WSS_URL").expect("BSC_WSS_URL must be set in .env file");
    let token_address = env::var("TOKEN_ADDRESS").expect("TOKEN_ADDRESS must be set in .env file");

    println!("\n🦀 BSC Token Streamer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Create formatter for displaying swaps
    let formatter = SwapFormatter::new();

    // Build and start streamer with auto-detection
    StreamerBuilder::from_wss(&wss_url)
        .await?
        .token_address(&token_address)
        .auto_detect() // Automatically detect platform and handle migration
        .on_swap(move |swap| {
            formatter.display(&swap);
        })
        .start()
        .await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down...");

    Ok(())
}
