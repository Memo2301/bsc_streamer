use bsc_streamer::MultiTokenStreamer;
use ethers::providers::{Provider, Ws};
use std::io::{self, Write};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("bsc_streamer=info,ethers=warn,ethers_providers::rpc::transports::ws=off")
        .init();

    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║         INTERACTIVE MULTI-TOKEN STREAMER                                ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    // Connect to BSC
    let wss_url = "wss://bsc.publicnode.com";
    println!("🔗 Connecting to BSC...");
    let provider = Provider::<Ws>::connect(wss_url).await?;
    let provider = Arc::new(provider);

    // Create multi-token streamer
    let streamer = MultiTokenStreamer::new(provider);
    println!("✅ Multi-token streamer initialized\n");

    println!("Available commands:");
    println!("  add <token_address>    - Add a token to monitor");
    println!("  remove <token_address> - Remove a token from monitoring");
    println!("  list                   - List all monitored tokens");
    println!("  count                  - Show number of monitored tokens");
    println!("  stop                   - Stop all monitoring and exit");
    println!("  help                   - Show this help message\n");

    // Interactive loop
    loop {
        print!("streamer> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0];

        match command {
            "add" => {
                if parts.len() < 2 {
                    println!("❌ Usage: add <token_address>");
                    continue;
                }
                let token = parts[1];
                
                match streamer.add_token(
                    token,
                    |swap| {
                        println!(
                            "💫 {} {} {} for {} {} | ${:.6} | Block {}",
                            swap.trade_type.as_str(),
                            swap.token.amount,
                            swap.token.symbol,
                            swap.base_token.amount,
                            swap.base_token.symbol,
                            swap.price.value,
                            swap.block_number
                        );
                    },
                    Some(|migration: bsc_streamer::MigrationEvent| {
                        println!(
                            "\n🎉 ═══════════════════════════════════════════════════════════\n\
                             MIGRATION DETECTED!\n\
                             Token: {:?}\n\
                             From: {}\n\
                             To: {}\n\
                             Pairs: {}\n\
                             Block: {}\n\
                             ═══════════════════════════════════════════════════════════\n",
                            migration.token_address,
                            migration.from_platform.as_str(),
                            migration.to_platform.as_str(),
                            migration.pair_count,
                            migration.block_number
                        );
                    })
                ).await {
                    Ok(_) => {},
                    Err(e) => println!("❌ Error: {}", e),
                }
            }
            "remove" => {
                if parts.len() < 2 {
                    println!("❌ Usage: remove <token_address>");
                    continue;
                }
                let token = parts[1];
                
                match streamer.remove_token(token).await {
                    Ok(_) => {},
                    Err(e) => println!("❌ Error: {}", e),
                }
            }
            "list" => {
                let tokens = streamer.list_tokens().await;
                if tokens.is_empty() {
                    println!("📭 No tokens currently monitored");
                } else {
                    println!("📋 Monitored tokens:");
                    for (i, token) in tokens.iter().enumerate() {
                        println!("  {}. {:?}", i + 1, token);
                    }
                }
            }
            "count" => {
                let count = streamer.token_count().await;
                println!("📊 Currently monitoring {} token(s)", count);
            }
            "stop" => {
                println!("\n🛑 Stopping all monitoring...");
                streamer.stop_all().await;
                println!("👋 Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  add <token_address>    - Add a token to monitor");
                println!("  remove <token_address> - Remove a token from monitoring");
                println!("  list                   - List all monitored tokens");
                println!("  count                  - Show number of monitored tokens");
                println!("  stop                   - Stop all monitoring and exit");
                println!("  help                   - Show this help message");
            }
            _ => {
                println!("❌ Unknown command: {}. Type 'help' for available commands.", command);
            }
        }
    }

    Ok(())
}

