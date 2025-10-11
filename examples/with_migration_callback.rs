use bsc_streamer::StreamerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: Monitor Four.meme token with migration notification
    
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .auto_detect()
        .on_swap(|swap| {
            // Handle swap events
            println!(
                "{} {} {} for {} {}",
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.token.symbol,
                swap.base_token.amount,
                swap.base_token.symbol
            );
        })
        .on_migration(|migration| {
            // Handle migration event
            println!("\n🎉 ════════════════════════════════════════");
            println!("   MIGRATION DETECTED!");
            println!("════════════════════════════════════════\n");
            println!("Token: {:?}", migration.token_address);
            println!("From: {}", migration.from_platform.as_str());
            println!("To: {}", migration.to_platform.as_str());
            println!("Block: {}", migration.block_number);
            println!("Tx: {:?}", migration.transaction_hash);
            println!("DEX Pairs Found: {}", migration.pair_count);
            
            for (i, pair_addr) in migration.pair_addresses.iter().enumerate() {
                println!("  Pair {}: {:?}", i + 1, pair_addr);
            }
            
            if let Some(timestamp) = &migration.timestamp {
                println!("Time: {}", timestamp);
            }
            
            println!("\n════════════════════════════════════════\n");
            
            // You can also send alerts, update databases, trigger actions, etc.
            // send_telegram_alert(&migration);
            // database.update_token_status(migration.token_address, "migrated");
            // trading_bot.adjust_strategy_for_dex();
        })
        .start()
        .await?;

    Ok(())
}

