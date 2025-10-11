use bsc_streamer::find_token_location;
use ethers::providers::{Provider, Ws};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: Find where a token is currently trading
    
    let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
    let token_address = "0x...";

    println!("🔍 Finding token location...\n");
    
    let location = find_token_location(Arc::new(provider), token_address).await?;

    println!("Token Information:");
    println!("  On Bonding Curve: {}", location.on_bonding_curve);
    println!("  DEX Pairs: {}", location.dex_pairs);
    println!("  Available on:");
    
    for platform in &location.platforms {
        println!("    - {}", platform.as_str());
    }

    if location.platforms.is_empty() {
        println!("    ⚠️  Token not found on any supported platform");
    }

    Ok(())
}

