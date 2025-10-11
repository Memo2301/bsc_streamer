use bsc_streamer::{Platform, StreamerBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: Manually specify platform if you know where the token is
    
    // Monitor a token on Four.meme bonding curve
    // (will auto-switch to DEX when migration happens)
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .platform(Platform::FourMemeBondingCurve)
        .on_swap(|swap| {
            println!(
                "[{}] {} {} @ {} {}",
                swap.platform.as_str(),
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.price.value,
                swap.base_token.symbol
            );
        })
        .start()
        .await?;

    Ok(())
}

