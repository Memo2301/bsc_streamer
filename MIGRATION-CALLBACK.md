# Migration Callback Feature 📢

## Overview

The BSC Token Streamer now supports **migration event notifications**, allowing users to be notified when a token migrates from Four.meme bonding curve to PancakeSwap.

## Features

- ✅ **Optional callback** - Use `on_migration()` to receive notifications
- ✅ **Complete event data** - Access to all migration details
- ✅ **Event-driven** - Triggered immediately when migration detected (1-3 seconds)
- ✅ **Zero overhead** - If callback not provided, no additional processing
- ✅ **Type-safe** - Full Rust type safety and compile-time checks

## Usage

### Basic Usage

```rust
use bsc_streamer::StreamerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    StreamerBuilder::from_wss("wss://bsc.publicnode.com")
        .await?
        .token_address("0x...")
        .auto_detect()
        .on_swap(|swap| {
            println!("Swap: {:?}", swap);
        })
        .on_migration(|migration| {
            println!("🎉 Migration detected!");
            println!("From: {}", migration.from_platform.as_str());
            println!("To: {}", migration.to_platform.as_str());
            println!("Pairs: {}", migration.pair_count);
        })
        .start()
        .await?;

    Ok(())
}
```

### Without Migration Callback

If you don't need migration notifications, simply omit `.on_migration()`:

```rust
StreamerBuilder::from_wss("wss://bsc.publicnode.com")
    .await?
    .token_address("0x...")
    .auto_detect()
    .on_swap(|swap| {
        println!("Swap: {:?}", swap);
    })
    .start() // No migration callback
    .await?;
```

The auto-migration will still work—you just won't be notified when it happens.

## MigrationEvent Structure

```rust
pub struct MigrationEvent {
    /// Token address that migrated
    pub token_address: Address,
    
    /// Source platform (usually FourMemeBondingCurve)
    pub from_platform: Platform,
    
    /// Destination platform (usually PancakeSwap)
    pub to_platform: Platform,
    
    /// Transaction hash of the PairCreated transaction
    pub transaction_hash: H256,
    
    /// Block number when migration occurred
    pub block_number: u64,
    
    /// Timestamp of migration
    pub timestamp: Option<String>,
    
    /// New DEX pair addresses created
    pub pair_addresses: Vec<Address>,
    
    /// Number of pairs found on DEX
    pub pair_count: usize,
}
```

### Helper Method

```rust
// Get formatted message
let message = migration.as_message();
// Returns: "🎉 MIGRATION DETECTED! Token migrated from Four.meme Bonding Curve to PancakeSwap V2 at block 45678901. Found 1 DEX pair(s)."
```

## Use Cases

### 1. Send Alerts

```rust
.on_migration(|migration| {
    // Telegram
    send_telegram_message(&format!(
        "🎉 Token {:?} migrated to PancakeSwap!",
        migration.token_address
    ));
    
    // Discord
    send_discord_webhook(&migration);
    
    // Email
    send_email_alert(&migration);
})
```

### 2. Update Database

```rust
.on_migration(|migration| {
    database.execute(
        "UPDATE tokens SET status = $1, migrated_at = $2 WHERE address = $3",
        &["migrated", &migration.timestamp, &migration.token_address]
    );
    
    database.execute(
        "INSERT INTO migration_events (token, tx_hash, block, pairs) VALUES ($1, $2, $3, $4)",
        &[&migration.token_address, &migration.transaction_hash, &migration.block_number, &migration.pair_count]
    );
})
```

### 3. Trading Bot Strategy Adjustment

```rust
.on_migration(|migration| {
    // Stop bonding curve strategy
    trading_bot.stop_bonding_curve_strategy();
    
    // Start DEX strategy
    trading_bot.start_dex_strategy(
        migration.token_address,
        &migration.pair_addresses
    );
    
    // Update liquidity tracking
    for pair in &migration.pair_addresses {
        trading_bot.add_liquidity_source(*pair);
    }
})
```

### 4. Analytics & Logging

```rust
.on_migration(|migration| {
    // Log to analytics service
    analytics.track_event("token_migration", json!({
        "token": migration.token_address,
        "from": migration.from_platform.as_str(),
        "to": migration.to_platform.as_str(),
        "block": migration.block_number,
        "pairs": migration.pair_count,
    }));
    
    // Update metrics
    metrics.increment_counter("migrations_detected");
    
    // Store for historical analysis
    history_db.insert_migration(&migration);
})
```

### 5. Webhook Notifications

```rust
.on_migration(|migration| {
    let client = reqwest::Client::new();
    
    tokio::spawn(async move {
        let _ = client
            .post("https://your-webhook.com/migration")
            .json(&migration)
            .send()
            .await;
    });
})
```

### 6. Multi-Channel Notifications

```rust
.on_migration(|migration| {
    println!("{}", migration.as_message());
    
    // Spawn async tasks for external notifications
    let migration_clone = migration.clone();
    tokio::spawn(async move {
        send_telegram_alert(&migration_clone).await;
    });
    
    let migration_clone = migration.clone();
    tokio::spawn(async move {
        send_discord_alert(&migration_clone).await;
    });
    
    let migration_clone = migration.clone();
    tokio::spawn(async move {
        update_database(&migration_clone).await;
    });
})
```

## How It Works

1. **Detection**: When auto-detection is enabled, the streamer watches the PancakeSwap V2 Factory contract for `PairCreated` events
2. **Matching**: Filters events to find pairs that include the target token
3. **Event Creation**: When a match is found, creates a `MigrationEvent` with complete details
4. **Callback Execution**: Calls user's migration callback (if provided)
5. **Transition**: Automatically switches from bonding curve to DEX monitoring
6. **Continuation**: Swap events continue to flow seamlessly

## Timeline

```
Time: 0s
  └─ Token on bonding curve
  └─ Monitoring Transfer events
  └─ Watching Factory for PairCreated

Time: 10s (example)
  └─ PairCreated event detected!
  └─ MigrationEvent created
  └─ User callback triggered       ← YOU ARE NOTIFIED HERE
  └─ Switch to DEX monitoring
  
Time: 10.5s
  └─ Now monitoring DEX Swap events
  └─ Zero downtime, seamless transition
```

## Example Output

When a migration occurs:

```
🟢 BUY TokenX ➡️ [Four.meme Bonding Curve]
   Amount: 1,000,000.00 TokenX
   For: 0.01 BNB
   
[... more bonding curve trades ...]

🎉 MIGRATION DETECTED! PairCreated event received!
🔄 Switching from bonding curve to DEX monitoring...

🎉 ════════════════════════════════════════
   MIGRATION DETECTED!
════════════════════════════════════════

Token: 0xb3bc1dfc3446a2e9c6a2133ad860f00932d34444
From: Four.meme Bonding Curve
To: PancakeSwap V2
Block: 45678901
Tx: 0x1234abcd...
DEX Pairs Found: 1
  Pair 1: 0x9876fedc...
Time: 2025-10-09T14:30:45+00:00

════════════════════════════════════════

📡 Now monitoring 1 DEX pair(s)

  ✅ Listening to WBNB pair: 0x9876fedc...

✨ DEX monitoring is now active!

🟢 BUY TokenX from WBNB ➡️ [PancakeSwap V2]
   Amount: 500,000.00 TokenX
   For: 0.005 WBNB
```

## Benefits

### For Trading Bots
- **Instant Strategy Adjustment** - Switch strategies immediately upon migration
- **No Manual Intervention** - Fully automated transition
- **Preserve History** - Log migration events for analysis

### For Alert Systems
- **Real-time Notifications** - Notify users the moment migration happens
- **Multi-channel** - Send to Telegram, Discord, Email, SMS, etc.
- **Rich Context** - Include all relevant details in notifications

### For Analytics Platforms
- **Track Migrations** - Build migration history databases
- **Market Analysis** - Analyze migration patterns and timing
- **Token Lifecycle** - Track complete token journey

### For Portfolio Managers
- **Position Updates** - Update tracked positions automatically
- **Liquidity Tracking** - Track new liquidity sources
- **Performance Metrics** - Measure performance across platforms

## FAQ

### Q: Is the migration callback required?
**A:** No, it's completely optional. Auto-migration will work without it.

### Q: When exactly is the callback triggered?
**A:** Immediately after the PairCreated event is detected, before switching to DEX monitoring.

### Q: Can I use async code in the callback?
**A:** Yes! Spawn tokio tasks for async operations:
```rust
.on_migration(|migration| {
    tokio::spawn(async move {
        send_alert(&migration).await;
    });
})
```

### Q: Will the callback block swap events?
**A:** No, the callback runs in its own task and won't block swap event processing.

### Q: Can I access external state in the callback?
**A:** Yes, use `Arc` and cloning:
```rust
let database = Arc::new(my_database);
let db_clone = database.clone();

.on_migration(move |migration| {
    db_clone.log_migration(&migration);
})
```

### Q: What if I want to stop monitoring after migration?
**A:** You can exit the program or store a cancellation token:
```rust
let cancel_token = Arc::new(AtomicBool::new(false));
let cancel_clone = cancel_token.clone();

.on_migration(move |_| {
    cancel_clone.store(true, Ordering::SeqCst);
    std::process::exit(0); // Or handle gracefully
})
```

## Complete Example

See [`examples/with_migration_callback.rs`](examples/with_migration_callback.rs) for a complete working example.

Run it:
```bash
cargo run --example with_migration_callback
```

## API Reference

See [README-LIBRARY.md](README-LIBRARY.md) for complete API documentation.

## Changelog

- **v1.1.0** - Added migration callback support
- **v1.0.0** - Initial release with auto-migration detection

---

**Need help?** Open an issue on GitHub or check the [README-LIBRARY.md](README-LIBRARY.md) for more examples.
