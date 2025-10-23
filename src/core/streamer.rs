use anyhow::{anyhow, Result};
use ethers::{
    providers::{Middleware, StreamExt},
    types::{Address, Filter, H256, U64},
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::{get_bonding_curve_address, get_factory_address};
use crate::core::{pair_finder::PairFinder, swap_parser::SwapParser, token_info::TokenInfoCache};
use crate::types::{MigrationEvent, Platform, SwapEvent};

const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const SWAP_V2_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
// PancakeSwap V3 Swap event with protocolFees:
// Swap(address indexed sender,address indexed recipient,int256 amount0,int256 amount1,uint160 sqrtPriceX96,uint128 liquidity,int24 tick,uint128 protocolFeesToken0,uint128 protocolFeesToken1)
const SWAP_V3_TOPIC: &str = "0xbd3314738ef3546c5fb0b02c304196e934faf0cfafd027f586e5e2970ad0c47a";
const PAIR_CREATED_TOPIC: &str = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9";

pub struct SwapStreamer<M> {
    provider: Arc<M>,
    pair_finder: PairFinder<M>,
    swap_parser: SwapParser<M>,
    is_streaming: bool,
}

impl<M: Middleware + 'static> SwapStreamer<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self {
            pair_finder: PairFinder::new(provider.clone()),
            swap_parser: SwapParser::new(provider.clone()),
            provider,
            is_streaming: false,
        }
    }

    pub async fn start<F>(&mut self, token_address_str: &str, callback: F) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        M::Provider: ethers::providers::PubsubClient,
    {
        self.start_with_migration_callback(token_address_str, callback, Option::<fn(MigrationEvent)>::None).await
    }

    pub async fn start_with_migration_callback<F, G>(
        &mut self,
        token_address_str: &str,
        swap_callback: F,
        migration_callback: Option<G>,
    ) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        G: Fn(MigrationEvent) + Send + Sync + 'static,
        M::Provider: ethers::providers::PubsubClient,
    {
        // Call the cancel-aware version with a dummy token that never cancels
        self.start_with_migration_callback_and_cancel(
            token_address_str,
            swap_callback,
            migration_callback,
            CancellationToken::new(), // Never cancelled
        ).await
    }

    pub async fn start_with_migration_callback_and_cancel<F, G>(
        &mut self,
        token_address_str: &str,
        swap_callback: F,
        migration_callback: Option<G>,
        cancel_token: CancellationToken,
    ) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        G: Fn(MigrationEvent) + Send + Sync + 'static,
        M::Provider: ethers::providers::PubsubClient,
    {
        if self.is_streaming {
            log::warn!("⚠️  Streamer is already running");
            return Ok(());
        }

        let token_address = Address::from_str(token_address_str)?;

        log::debug!("🚀 Starting swap event streamer for token: {}", token_address_str);

        // CRITICAL FIX: Check for DEX pairs FIRST before checking bonding curve
        // This prevents migrated tokens from being incorrectly detected as still on bonding curve
        // (The bonding curve check looks at historical transfers which may include pre-migration activity)
        let pairs = self.pair_finder.find_pairs(token_address).await?;

        if !pairs.is_empty() {
            // Token has DEX pairs - monitor DEX
            log::debug!("📡 Monitoring {} DEX pair(s) for real-time swaps", pairs.len());

            self.is_streaming = true;

            // Wrap callback in Arc once
            let callback = Arc::new(swap_callback);

            // Monitor each pair
            for pair_info in pairs {
                // Use correct swap topic based on pool type
                let swap_topic = if pair_info.is_v3 {
                    H256::from_str(SWAP_V3_TOPIC)?
                } else {
                    H256::from_str(SWAP_V2_TOPIC)?
                };
                
                let pool_type = if pair_info.is_v3 { "V3" } else { "V2" };
                
                // Watch for new events only (from latest block forward)
                let filter = Filter::new()
                    .address(pair_info.pair_address)
                    .topic0(swap_topic);

                let parser = self.swap_parser.clone();
                let pair_info_clone = pair_info.clone();
                let callback_clone = callback.clone();
                let cancel_clone = cancel_token.clone();

                tokio::spawn(async move {
                    log::info!("🔄 [SWAP_STREAMER] Starting {} subscription for pair {:?}", pool_type, pair_info_clone.pair_address);
                    
                    // Use subscribe_logs for WebSocket providers (eth_subscribe instead of polling)
                    match parser.provider.subscribe_logs(&filter).await {
                        Ok(mut stream) => {
                            log::info!("✅ [SWAP_STREAMER] {} subscription created successfully for pair {:?}", pool_type, pair_info_clone.pair_address);
                            
                            let mut events_received = 0;
                            let mut events_parsed = 0;
                            let mut events_failed = 0;
                            let mut last_log_time = std::time::Instant::now();
                            let start_time = std::time::Instant::now();
                            
                            loop {
                                // Log heartbeat every 30 seconds to show subscription is alive
                                if last_log_time.elapsed().as_secs() >= 30 {
                                    let uptime = start_time.elapsed();
                                    let rate = if uptime.as_secs() > 0 {
                                        events_received as f64 / uptime.as_secs() as f64
                                    } else {
                                        0.0
                                    };
                                    
                                    log::info!("💓 [SWAP_STREAMER] {} pair {:?} - Received: {}, Parsed: {}, Failed: {}, Rate: {:.2}/s", 
                                        pool_type, pair_info_clone.pair_address, events_received, events_parsed, events_failed, rate);
                                    last_log_time = std::time::Instant::now();
                                }
                                
                                tokio::select! {
                                    // Listen for cancel signal
                                    _ = cancel_clone.cancelled() => {
                                        log::info!("🛑 [SWAP_STREAMER] {} subscription cancelled - Received: {}, Parsed: {}, Failed: {}", 
                                            pool_type, events_received, events_parsed, events_failed);
                                        break;
                                    }
                                    // Process stream events
                                    log_option = stream.next() => {
                                        match log_option {
                                            Some(log) => {
                                                events_received += 1;
                                                let receive_time = std::time::Instant::now();
                                                
                                                // Log block number to detect batching
                                                if events_received == 1 || events_received % 100 == 0 {
                                                    log::info!("📊 [SWAP_STREAMER] Event #{}: block={:?}, tx={:?}", 
                                                        events_received, log.block_number, log.transaction_hash);
                                                }
                                                
                                                log::debug!("📥 [SWAP_STREAMER] Received {} log #{} for pair {:?} - tx: {:?}", 
                                                    pool_type, events_received, pair_info_clone.pair_address, log.transaction_hash);
                                                
                                                let parse_start = std::time::Instant::now();
                                                match parser.parse_swap_event(&log, &pair_info_clone).await {
                                                    Ok(swap) => {
                                                        events_parsed += 1;
                                                        let parse_duration = parse_start.elapsed();
                                                        log::debug!("✅ [SWAP_STREAMER] Parsed {} event #{} in {:?}: {:?} {} @ {:.10} {}", 
                                                            pool_type, events_received, parse_duration, swap.trade_type, swap.token.amount, 
                                                            swap.price.value, swap.price.base_token);
                                                        
                                                        let callback_start = std::time::Instant::now();
                                                        callback_clone(swap);
                                                        let callback_duration = callback_start.elapsed();
                                                        
                                                        let total_duration = receive_time.elapsed();
                                                        if total_duration.as_millis() > 500 {
                                                            log::warn!("⚠️  [SWAP_STREAMER] Slow event processing: parse={:?}, callback={:?}, total={:?}", 
                                                                parse_duration, callback_duration, total_duration);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        events_failed += 1;
                                                        log::error!("❌ [SWAP_STREAMER] Failed to parse {} swap event: {}", pool_type, e);
                                                    }
                                                }
                                            }
                                            None => {
                                                log::warn!("⚠️ [SWAP_STREAMER] {} stream ended - Received: {}, Parsed: {}, Failed: {}", 
                                                    pool_type, events_received, events_parsed, events_failed);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("❌ [SWAP_STREAMER] Failed to create {} subscription for pair {:?}: {}", pool_type, pair_info_clone.pair_address, e);
                            log::error!("   Error details: {:?}", e);
                        }
                    }
                });

                log::debug!("  ✅ Listening to {} {} pair: {:?}", pool_type, pair_info.base_token_symbol, pair_info.pair_address);
            }

            log::debug!("✨ Streamer is now active. Waiting for swap events...");

            return Ok(());
        }

        // No DEX pairs found - check if token is on Four.meme bonding curve
        if let Ok(has_activity) = self.check_bonding_curve(&token_address).await {
            if has_activity {
                self.is_streaming = true;
                self.start_bonding_curve_with_migration_detection_and_callback(
                    token_address,
                    swap_callback,
                    migration_callback,
                    cancel_token.clone(),
                )
                .await?;
                return Ok(());
            }
        }

        // No DEX pairs and not on bonding curve
        return Err(anyhow!("No trading pairs found on DEX and not on bonding curve"));
    }

    /// Public method to check if a token is on the bonding curve (for library users)
    pub async fn check_bonding_curve_public(&self, token_address: &Address) -> Result<bool> {
        self.check_bonding_curve(token_address).await
    }

    async fn check_bonding_curve(&self, token_address: &Address) -> Result<bool> {
        log::debug!("🔍 Checking Four.meme bonding curve for token...");

        let bonding_curve = get_bonding_curve_address();
        let transfer_topic = H256::from_str(TRANSFER_TOPIC)?;

        // Check recent blocks for activity (limit to 5000 blocks to avoid RPC limits)
        let current_block = self.provider.get_block_number().await?;
        let from_block = current_block.saturating_sub(U64::from(5000));

        let filter = Filter::new()
            .address(*token_address)
            .topic0(transfer_topic)
            .from_block(from_block)
            .to_block(current_block);

        let logs = self.provider.get_logs(&filter).await.unwrap_or_else(|_| vec![]);

        // Check if any transfers involve the bonding curve
        for log in logs.iter().take(50) {
            if log.topics.len() >= 3 {
                let from = Address::from(log.topics[1]);
                let to = Address::from(log.topics[2]);

                if from == bonding_curve || to == bonding_curve {
                    log::debug!("  ✅ Found Four.meme bonding curve activity");
                    return Ok(true);
                }
            }
        }

        log::debug!("  ⚪ No Four.meme bonding curve activity found");
        Ok(false)
    }

    async fn start_bonding_curve_with_migration_detection_and_callback<F, G>(
        &self,
        token_address: Address,
        swap_callback: F,
        migration_callback: Option<G>,
        cancel_token: CancellationToken,
    ) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        G: Fn(MigrationEvent) + Send + Sync + 'static,
        M::Provider: ethers::providers::PubsubClient,
    {
        let bonding_curve = get_bonding_curve_address();
        let factory_address = get_factory_address();
        let transfer_topic = H256::from_str(TRANSFER_TOPIC)?;
        let pair_created_topic = H256::from_str(PAIR_CREATED_TOPIC)?;

        // Create channel for migration detection
        let (migration_tx, mut migration_rx) = mpsc::channel::<(H256, u64)>(1);

        // Watch for Transfer events on the token (bonding curve trades)
        let transfer_filter = Filter::new()
            .address(token_address)
            .topic0(transfer_topic);

        let parser = self.swap_parser.clone();
        let swap_callback = Arc::new(swap_callback);
        let migration_callback = migration_callback.map(Arc::new);

        log::debug!("  ✅ Listening to Four.meme bonding curve: {:?}", bonding_curve);
        log::debug!("  🔍 Watching PancakeSwap Factory for PairCreated event");
        log::debug!("✨ Streamer is now active. Waiting for bonding curve trades...");

        // Spawn bonding curve event listener
        let callback_clone = swap_callback.clone();
        let cancel_clone = cancel_token.clone();
        tokio::spawn(async move {
            // Use subscribe_logs for WebSocket providers (eth_subscribe instead of polling)
            if let Ok(mut stream) = parser.provider.subscribe_logs(&transfer_filter).await {
                loop {
                    tokio::select! {
                        _ = cancel_clone.cancelled() => {
                            log::debug!("🛑 [BONDING_CURVE] Transfer event listener cancelled");
                            break;
                        }
                        log_option = stream.next() => {
                            match log_option {
                                Some(log) => {
                                    if log.topics.len() >= 3 {
                                        let from = Address::from(log.topics[1]);
                                        let to = Address::from(log.topics[2]);

                                        if from == bonding_curve || to == bonding_curve {
                                            if let Ok(Some(swap)) = parser
                                                .parse_bonding_curve_event(&log, token_address, bonding_curve)
                                                .await
                                            {
                                                callback_clone(swap);
                                            }
                                        }
                                    }
                                }
                                None => {
                                    log::warn!("⚠️ [BONDING_CURVE] Transfer stream ended");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        // Spawn PairCreated event listener on Factory
        let provider_clone = self.provider.clone();
        let pair_finder = PairFinder::new(provider_clone.clone());
        let cancel_clone2 = cancel_token.clone();
        
        tokio::spawn(async move {
            // Watch for PairCreated events from the Factory
            // PairCreated(address indexed token0, address indexed token1, address pair, uint)
            // We need to check if either token0 or token1 matches our target token
            let filter = Filter::new()
                .address(factory_address)
                .topic0(pair_created_topic);
            
            // Use subscribe_logs for WebSocket providers (eth_subscribe instead of polling)
            if let Ok(mut stream) = provider_clone.subscribe_logs(&filter).await {
                loop {
                    tokio::select! {
                        _ = cancel_clone2.cancelled() => {
                            log::debug!("🛑 [BONDING_CURVE] PairCreated event listener cancelled");
                            break;
                        }
                        log_option = stream.next() => {
                            match log_option {
                                Some(log) => {
                                    if log.topics.len() >= 3 {
                                        let token0 = Address::from(log.topics[1]);
                                        let token1 = Address::from(log.topics[2]);
                                        
                                        // Check if either token matches our target token
                                        if token0 == token_address || token1 == token_address {
                                            log::info!("🎉 MIGRATION DETECTED! PairCreated event received!");
                                            log::info!("🔄 Switching from bonding curve to DEX monitoring...");
                                            
                                            // Send transaction hash and block number for migration event
                                            if let (Some(tx_hash), Some(block_num)) = (log.transaction_hash, log.block_number) {
                                                let _ = migration_tx.send((tx_hash, block_num.as_u64())).await;
                                                break;
                                            }
                                        }
                                    }
                                }
                                None => {
                                    log::warn!("⚠️ [BONDING_CURVE] PairCreated stream ended");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        // Wait for migration event and start DEX monitoring
        let parser_for_dex = self.swap_parser.clone();
        let provider_for_migration = self.provider.clone();
        tokio::spawn(async move {
            if let Some((tx_hash, block_number)) = migration_rx.recv().await {
                // Get full pair info
                let pairs = pair_finder.find_pairs(token_address).await.unwrap_or_else(|_| vec![]);
                
                if pairs.is_empty() {
                    log::warn!("⚠️  Migration detected but couldn't fetch pair details");
                    return;
                }

                // Create and emit migration event
                if let Some(migration_cb) = &migration_callback {
                    let pair_addresses: Vec<Address> = pairs.iter().map(|p| p.pair_address).collect();
                    
                    // Get timestamp
                    let timestamp = if let Ok(Some(block)) = provider_for_migration.get_block(block_number).await {
                        block.timestamp
                            .as_u64()
                            .checked_mul(1000)
                            .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
                            .map(|dt| dt.to_rfc3339())
                    } else {
                        None
                    };

                    let migration_event = MigrationEvent {
                        token_address,
                        from_platform: Platform::FourMemeBondingCurve,
                        to_platform: Platform::PancakeSwap,
                        transaction_hash: tx_hash,
                        block_number,
                        timestamp,
                        pair_addresses: pair_addresses.clone(),
                        pair_count: pairs.len(),
                    };
                    
                    migration_cb(migration_event);
                }
                
                // Start DEX monitoring
                log::info!("📡 Now monitoring {} DEX pair(s)", pairs.len());
                
                for pair_info in pairs {
                    let swap_topic = if pair_info.is_v3 {
                        H256::from_str(SWAP_V3_TOPIC).unwrap()
                    } else {
                        H256::from_str(SWAP_V2_TOPIC).unwrap()
                    };
                    
                    let pool_type = if pair_info.is_v3 { "V3" } else { "V2" };
                    
                    let filter = Filter::new()
                        .address(pair_info.pair_address)
                        .topic0(swap_topic);
                    
                    let parser_clone = parser_for_dex.clone();
                    let pair_info_clone = pair_info.clone();
                    let callback_clone = swap_callback.clone();
                    let cancel_clone3 = cancel_token.clone();
                    
                    tokio::spawn(async move {
                        // Use subscribe_logs for WebSocket providers (eth_subscribe instead of polling)
                        if let Ok(mut stream) = parser_clone.provider.subscribe_logs(&filter).await {
                            loop {
                                tokio::select! {
                                    _ = cancel_clone3.cancelled() => {
                                        log::debug!("🛑 [MIGRATION_DEX] Swap event listener cancelled for pair {:?}", pair_info_clone.pair_address);
                                        break;
                                    }
                                    log_option = stream.next() => {
                                        match log_option {
                                            Some(log) => {
                                                if let Ok(swap) = parser_clone.parse_swap_event(&log, &pair_info_clone).await {
                                                    callback_clone(swap);
                                                }
                                            }
                                            None => {
                                                log::warn!("⚠️ [MIGRATION_DEX] Stream ended for pair {:?}", pair_info_clone.pair_address);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                    
                    log::debug!("  ✅ Listening to {} {} pair: {:?}", pool_type, pair_info.base_token_symbol, pair_info.pair_address);
                }
                
                log::info!("✨ DEX monitoring is now active!");
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) {
        if self.is_streaming {
            log::info!("🛑 Stopping streamer...");
            self.is_streaming = false;
            log::info!("✅ Streamer stopped.");
        }
    }
}

// Add Clone for SwapParser
impl<M: Middleware + 'static> Clone for SwapParser<M> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            token_cache: TokenInfoCache::new(self.provider.clone()),
        }
    }
}

