# BSC Token Streamer 🦀

A high-performance, event-driven Rust library for monitoring BSC token swap events with automatic Four.meme bonding curve to PancakeSwap migration detection.

## Features

- ⚡ **Real-time Event Streaming** - WebSocket-based, low-latency swap event monitoring
- 🎯 **Four.meme Support** - Full bonding curve trade monitoring with accurate price calculation
- 🔄 **Auto-Migration Detection** - Instant detection via PairCreated events (1-3 second latency)
- 📢 **Migration Notifications** - Optional callbacks to be notified when tokens migrate
- 🔀 **Multi-Token Streaming** - Add/remove tokens dynamically while running (NEW!)
- 🔍 **Token Discovery** - Find where any token is currently trading
- 💰 **Price Tracking** - Real-time price calculation, change detection, and session statistics
- 🛡️ **Production Ready** - Type-safe, memory-safe, with graceful error handling
- 📦 **Easy to Use** - Simple builder pattern API

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
bsc_streamer = "1.0"
ethers = "2.0"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### Auto-Detection (Recommended)

Let the library automatically detect where your token is trading:

```rust
use bsc_streamer::StreamerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .auto_detect() // Automatically finds platform and handles migration
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
```

### With Migration Notifications

Get notified when a token migrates from Four.meme to PancakeSwap:

```rust
use bsc_streamer::StreamerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        .on_migration(|migration| {
            println!("🎉 Token migrated from {} to {}!",
                migration.from_platform.as_str(),
                migration.to_platform.as_str()
            );
            println!("Found {} DEX pair(s)", migration.pair_count);
            
            // Send alerts, update database, adjust trading strategy, etc.
            send_telegram_alert(&migration);
            database.mark_as_migrated(migration.token_address);
        })
        .start()
        .await?;

    Ok(())
}
```

### Manual Platform Selection

If you know where your token is trading:

```rust
use bsc_streamer::{Platform, StreamerBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .platform(Platform::FourMemeBondingCurve) // Or Platform::PancakeSwap
        .on_swap(|swap| {
            println!("Swap detected on {}", swap.platform.as_str());
        })
        .start()
        .await?;

    Ok(())
}
```

### Multi-Token Streaming

Monitor multiple tokens simultaneously with dynamic add/remove:

```rust
use bsc_streamer::MultiTokenStreamer;
use ethers::providers::{Provider, Ws};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
    let streamer = MultiTokenStreamer::new(Arc::new(provider));

    // Add Token A
    streamer.add_token(
        "0xTokenA...",
        |swap| println!("[A] {} {}", swap.trade_type.as_str(), swap.token.amount),
        Some(|migration| println!("[A] Migrated!"))
    ).await?;

    // Add Token B
    streamer.add_token(
        "0xTokenB...",
        |swap| println!("[B] {} {}", swap.trade_type.as_str(), swap.token.amount),
        None
    ).await?;

    // Later: remove Token A
    streamer.remove_token("0xTokenA...").await?;

    // List all monitored tokens
    let tokens = streamer.list_tokens().await;
    println!("Monitoring {} tokens", tokens.len());

    // Stop all
    streamer.stop_all().await;

    Ok(())
}
```

**[📖 Read full multi-token streaming guide →](MULTI-TOKEN-STREAMING.md)**

### Find Token Location

Discover where a token is trading before streaming:

```rust
use bsc_streamer::find_token_location;
use ethers::providers::{Provider, Ws};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
    
    let location = find_token_location(&provider, "0x...").await?;

    println!("On Bonding Curve: {}", location.on_bonding_curve);
    println!("DEX Pairs: {}", location.dex_pairs);
    
    for platform in location.platforms {
        println!("Available on: {}", platform.as_str());
    }

    Ok(())
}
```

### Using Your Own Provider

```rust
use bsc_streamer::StreamerBuilder;
use ethers::providers::{Provider, Ws};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your own provider with custom configuration
    let provider = Provider::<Ws>::connect("wss://your-custom-node.com").await?;
    
    StreamerBuilder::new(Arc::new(provider))
        .token_address("0x...")
        .auto_detect()
        .on_swap(|swap| {
            // Your custom logic
        })
        .start()
        .await?;

    Ok(())
}
```

## API Reference

### `StreamerBuilder`

Builder for configuring and starting a token swap event streamer.

#### Methods

- **`from_wss(url: &str)`** - Create streamer from WebSocket URL
- **`new(provider: Arc<Provider>)`** - Create streamer with existing provider
- **`token_address(address: &str)`** - Set token address to monitor
- **`auto_detect()`** - Enable automatic platform detection and migration
- **`platform(platform: Platform)`** - Manually specify platform
- **`on_swap(callback: F)`** - Set callback for swap events (required)
- **`on_migration(callback: G)`** - Set callback for migration events (optional)
- **`start()`** - Start streaming events

### `Platform`

Enum representing supported trading platforms:

```rust
pub enum Platform {
    PancakeSwap,
    FourMemeBondingCurve,
}
```

### `SwapEvent`

Struct containing swap event data:

```rust
pub struct SwapEvent {
    pub transaction_hash: H256,
    pub block_number: u64,
    pub timestamp: Option<String>,
    pub platform: Platform,
    pub trade_type: TradeType,
    pub token: TokenInfo,
    pub base_token: TokenInfo,
    pub price: PriceInfo,
    pub sender: Address,
    pub recipient: Address,
    pub pair_address: Option<Address>,
    pub bonding_curve_address: Option<Address>,
}
```

### `MigrationEvent`

Struct containing migration event data:

```rust
pub struct MigrationEvent {
    pub token_address: Address,
    pub from_platform: Platform,
    pub to_platform: Platform,
    pub transaction_hash: H256,
    pub block_number: u64,
    pub timestamp: Option<String>,
    pub pair_addresses: Vec<Address>,
    pub pair_count: usize,
}
```

Helper method:
```rust
migration.as_message() // Returns formatted string
```

### `find_token_location()`

Function to discover where a token is trading:

```rust
pub async fn find_token_location<M>(
    provider: &M,
    token_address: &str,
) -> Result<TokenLocation>
```

Returns:
```rust
pub struct TokenLocation {
    pub on_bonding_curve: bool,
    pub dex_pairs: usize,
    pub platforms: Vec<Platform>,
}
```

## Features Explained

### Auto-Migration Detection

When monitoring a Four.meme bonding curve token:

1. **Instant Detection** - Watches PancakeSwap Factory for PairCreated events
2. **1-3 Second Latency** - Event-driven, not polling-based
3. **Zero Downtime** - Seamless transition from bonding curve to DEX
4. **Automatic** - No manual intervention required
5. **Optional Notifications** - Get notified via `on_migration()` callback

Migration callback use cases:
- **Alerts** - Send Telegram, Discord, or email notifications
- **Database Updates** - Mark tokens as migrated in your database
- **Strategy Adjustments** - Update trading bot strategies automatically
- **Analytics** - Track migration events for analysis
- **Webhooks** - Trigger external services

### Efficient Event Filtering

- **Server-side filtering** - Only receives events for your token
- **Minimal bandwidth** - No unnecessary data transfer
- **Low CPU usage** - Event-driven architecture

### Price Calculation

- **Bonding Curve** - Extracts BNB amounts from event data
- **DEX** - Calculates from swap amounts
- **Session Stats** - Tracks price changes, highs, lows

## Examples

See the `examples/` directory for more usage patterns:

- `simple.rs` - Basic auto-detection usage
- `manual_platform.rs` - Manual platform selection
- `find_token.rs` - Token location discovery

Run examples:
```bash
cargo run --example simple
cargo run --example manual_platform
cargo run --example find_token
```

## Use Cases

### Trading Bots

Monitor tokens from launch to DEX without manual intervention:

```rust
StreamerBuilder::from_wss(wss_url)
    .await?
    .token_address(token)
    .auto_detect()
    .on_swap(|swap| {
        if swap.price.value > target_price {
            execute_trade();
        }
    })
    .start()
    .await?;
```

### Price Tracking Services

Maintain continuous price data:

```rust
let db = Database::connect();

StreamerBuilder::from_wss(wss_url)
    .await?
    .token_address(token)
    .auto_detect()
    .on_swap(move |swap| {
        db.insert_price(swap.token.address, swap.price.value);
    })
    .start()
    .await?;
```

### Alert Systems

Get notified about trades:

```rust
StreamerBuilder::from_wss(wss_url)
    .await?
    .token_address(token)
    .auto_detect()
    .on_swap(|swap| {
        if swap.trade_type == TradeType::Buy && swap.price.value > threshold {
            send_alert(format!("Large buy detected: {} BNB", swap.base_token.amount));
        }
    })
    .start()
    .await?;
```

## Performance

- **Memory:** ~15MB
- **CPU:** ~1%
- **Latency:** 0.5-1s for DEX, 1-3s for migration detection
- **Throughput:** Handles high-volume tokens efficiently

## Supported Platforms

- ✅ PancakeSwap V2
- ✅ Four.meme Bonding Curve
- 🚧 PancakeSwap V3 (coming soon)
- 🚧 Other DEXes (coming soon)

## Requirements

- Rust 1.70+
- BSC WebSocket RPC endpoint (e.g., `wss://bsc.publicnode.com`)

## Documentation

- [Migration Detection Guide](MIGRATION-DETECTION.md)
- [Four.meme Tokens](FOURMEME-TOKENS.md)
- [Troubleshooting](FIXED-BLOCK-RANGE-ERROR.md)

## Contributing

Contributions welcome! Please ensure:
- Code is formatted with `cargo fmt`
- No warnings from `cargo clippy`
- Tests pass with `cargo test`

## License

MIT

---

Built with Rust 🦀 for performance, safety, and reliability.

