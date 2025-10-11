use bsc_streamer::StreamerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple example: Auto-detect where token is trading
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .auto_detect()
        .on_swap(|swap| {
            println!(
                "{} {} {} for {} {}",
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.token.symbol,
                swap.base_token.amount,
                swap.base_token.symbol
            );
        })
        .start()
        .await?;

    Ok(())
}

