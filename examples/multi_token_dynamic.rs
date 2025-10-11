use bsc_streamer::MultiTokenStreamer;
use ethers::providers::{Provider, Ws};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("bsc_streamer=info,ethers=warn,ethers_providers::rpc::transports::ws=off")
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║         MULTI-TOKEN DYNAMIC STREAMER EXAMPLE                             ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    // Connect to BSC
    let wss_url = "wss://bsc.publicnode.com";
    let provider = Provider::<Ws>::connect(wss_url).await?;
    let provider = Arc::new(provider);

    // Create multi-token streamer
    let streamer = MultiTokenStreamer::new(provider);

    println!("📡 Multi-token streamer initialized\n");
    println!("This example will:");
    println!("  1. Add Token A");
    println!("  2. Wait 10 seconds");
    println!("  3. Add Token B");
    println!("  4. Wait 10 seconds");
    println!("  5. Remove Token A");
    println!("  6. Wait 10 seconds");
    println!("  7. Add Token C");
    println!("  8. List all monitored tokens");
    println!("  9. Wait 20 seconds");
    println!("  10. Stop all monitoring\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Token A
    let token_a = "0x76D394f4a9C3c30b3A80580F662B1046EcE04444";
    
    // Token B
    let token_b = "0xF368B32764A4b9e58Cf6da67bb454F7809bc4444";
    
    // Token C
    let token_c = "0x44440f83419DE123d7d411187aDb9962db017d03";

    // Add Token A
    println!("🔵 STEP 1: Adding Token A...");
    streamer.add_token(
        token_a,
        |swap| {
            println!(
                "🔵 [Token A] {} {} {} for {} {}",
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.token.symbol,
                swap.base_token.amount,
                swap.base_token.symbol
            );
        },
        Some(|migration: bsc_streamer::MigrationEvent| {
            println!("🔵 [Token A] 🎉 MIGRATED from {} to {}!",
                migration.from_platform.as_str(),
                migration.to_platform.as_str()
            );
        })
    ).await?;

    // Wait 10 seconds
    println!("\n⏰ Waiting 10 seconds...\n");
    sleep(Duration::from_secs(10)).await;

    // Add Token B
    println!("🟢 STEP 2: Adding Token B...");
    streamer.add_token(
        token_b,
        |swap| {
            println!(
                "🟢 [Token B] {} {} {} for {} {}",
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.token.symbol,
                swap.base_token.amount,
                swap.base_token.symbol
            );
        },
        Some(|migration: bsc_streamer::MigrationEvent| {
            println!("🟢 [Token B] 🎉 MIGRATED from {} to {}!",
                migration.from_platform.as_str(),
                migration.to_platform.as_str()
            );
        })
    ).await?;

    // Show token count
    let count = streamer.token_count().await;
    println!("📊 Currently monitoring {} tokens\n", count);

    // Wait 10 seconds
    println!("⏰ Waiting 10 seconds...\n");
    sleep(Duration::from_secs(10)).await;

    // Remove Token A
    println!("🔴 STEP 3: Removing Token A...");
    streamer.remove_token(token_a).await?;

    let count = streamer.token_count().await;
    println!("📊 Currently monitoring {} tokens\n", count);

    // Wait 10 seconds
    println!("⏰ Waiting 10 seconds...\n");
    sleep(Duration::from_secs(10)).await;

    // Add Token C
    println!("🟣 STEP 4: Adding Token C...");
    streamer.add_token(
        token_c,
        |swap| {
            println!(
                "🟣 [Token C] {} {} {} for {} {}",
                swap.trade_type.as_str(),
                swap.token.amount,
                swap.token.symbol,
                swap.base_token.amount,
                swap.base_token.symbol
            );
        },
        Some(|migration: bsc_streamer::MigrationEvent| {
            println!("🟣 [Token C] 🎉 MIGRATED from {} to {}!",
                migration.from_platform.as_str(),
                migration.to_platform.as_str()
            );
        })
    ).await?;

    // List all tokens
    println!("\n📋 STEP 5: Listing all monitored tokens:");
    let tokens = streamer.list_tokens().await;
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}. {:?}", i + 1, token);
    }
    println!();

    let count = streamer.token_count().await;
    println!("📊 Total: {} tokens\n", count);

    // Wait 20 seconds
    println!("⏰ Monitoring for 20 seconds...\n");
    sleep(Duration::from_secs(20)).await;

    // Stop all
    println!("🛑 STEP 6: Stopping all token monitoring...");
    streamer.stop_all().await;

    println!("✅ Example completed!\n");

    Ok(())
}

