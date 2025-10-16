use anyhow::{anyhow, Result};
use ethers::{
    providers::{Middleware, StreamExt},
    types::{Address, Filter, H256, U64},
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::config::{get_bonding_curve_address, get_factory_address};
use crate::core::{pair_finder::PairFinder, swap_parser::SwapParser, token_info::TokenInfoCache};
use crate::types::{MigrationEvent, Platform, SwapEvent};

const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const SWAP_V2_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
const SWAP_V3_TOPIC: &str = "0x19b47279256b2a23a1665c810c8d55a1758940ee09377d4f8d26497a3577dc83";
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
    {
        if self.is_streaming {
            log::warn!("⚠️  Streamer is already running");
            return Ok(());
        }

        let token_address = Address::from_str(token_address_str)?;

        log::info!("🚀 Starting swap event streamer for token: {}", token_address_str);

        // Check if token is on Four.meme bonding curve
        if let Ok(has_activity) = self.check_bonding_curve(&token_address).await {
            if has_activity {
                log::info!("🎯 Token is on Four.meme bonding curve!");
                log::info!("📡 Monitoring bonding curve trades...");
                log::info!("🔄 Watching for PairCreated event to auto-switch to DEX");

                self.is_streaming = true;
                self.start_bonding_curve_with_migration_detection_and_callback(
                    token_address,
                    swap_callback,
                    migration_callback,
                )
                .await?;
                return Ok(());
            }
        }

        // Find DEX pairs
        let pairs = self.pair_finder.find_pairs(token_address).await?;

        if pairs.is_empty() {
            return Err(anyhow!("No trading pairs found on DEX and not on bonding curve"));
        }

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

            tokio::spawn(async move {
                log::info!("🔄 [SWAP_STREAMER] Starting {} subscription task for pair {:?}", pool_type, pair_info_clone.pair_address);
                
                match parser.provider.watch(&filter).await {
                    Ok(watcher) => {
                        log::info!("✅ [SWAP_STREAMER] {} subscription created successfully for pair {:?}", pool_type, pair_info_clone.pair_address);
                        let mut stream = watcher.stream();
                        
                        let mut event_count = 0;
                        while let Some(log) = stream.next().await {
                            event_count += 1;
                            log::debug!("📥 [SWAP_STREAMER] Received {} log #{} for pair {:?}", pool_type, event_count, pair_info_clone.pair_address);
                            
                            match parser.parse_swap_event(&log, &pair_info_clone).await {
                                Ok(swap) => {
                                    log::info!("✅ [SWAP_STREAMER] Successfully parsed {} swap event #{}", pool_type, event_count);
                                    callback_clone(swap);
                                }
                                Err(e) => {
                                    log::error!("❌ [SWAP_STREAMER] Failed to parse {} swap event: {}", pool_type, e);
                                }
                            }
                        }
                        
                        log::warn!("⚠️ [SWAP_STREAMER] {} stream ended for pair {:?} after {} events", pool_type, pair_info_clone.pair_address, event_count);
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

        Ok(())
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
    ) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        G: Fn(MigrationEvent) + Send + Sync + 'static,
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
        tokio::spawn(async move {
            if let Ok(watcher) = parser.provider.watch(&transfer_filter).await {
                let mut stream = watcher.stream();
                while let Some(log) = stream.next().await {
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
            }
        });

        // Spawn PairCreated event listener on Factory
        let provider_clone = self.provider.clone();
        let pair_finder = PairFinder::new(provider_clone.clone());
        
        tokio::spawn(async move {
            // Watch for PairCreated events from the Factory
            // PairCreated(address indexed token0, address indexed token1, address pair, uint)
            // We need to check if either token0 or token1 matches our target token
            let filter = Filter::new()
                .address(factory_address)
                .topic0(pair_created_topic);
            
            if let Ok(watcher) = provider_clone.watch(&filter).await {
                let mut stream = watcher.stream();
                while let Some(log) = stream.next().await {
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
                    
                    tokio::spawn(async move {
                        if let Ok(watcher) = parser_clone.provider.watch(&filter).await {
                            let mut stream = watcher.stream();
                            while let Some(log) = stream.next().await {
                                if let Ok(swap) = parser_clone.parse_swap_event(&log, &pair_info_clone).await {
                                    callback_clone(swap);
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

