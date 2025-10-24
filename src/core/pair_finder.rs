use anyhow::Result;
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::Middleware,
    types::Address,
};
use std::sync::Arc;

use crate::config::{get_base_tokens, get_factory_address, get_v3_factory_address};
use crate::types::PairInfo;

// Minimum liquidity threshold in USD
const MIN_LIQUIDITY_USD: f64 = 5000.0;

const FACTORY_V2_ABI: &str = r#"[
    {"constant":true,"inputs":[{"name":"tokenA","type":"address"},{"name":"tokenB","type":"address"}],"name":"getPair","outputs":[{"name":"pair","type":"address"}],"type":"function"}
]"#;

const FACTORY_V3_ABI: &str = r#"[
    {"constant":true,"inputs":[{"name":"tokenA","type":"address"},{"name":"tokenB","type":"address"},{"name":"fee","type":"uint24"}],"name":"getPool","outputs":[{"name":"pool","type":"address"}],"type":"function"}
]"#;

// PancakeSwap V3 fee tiers (in basis points)
const V3_FEE_TIERS: [u32; 4] = [
    100,   // 0.01%
    500,   // 0.05%
    2500,  // 0.25%
    10000, // 1.00%
];

pub struct PairFinder<M> {
    provider: Arc<M>,
}

impl<M: Middleware + 'static> PairFinder<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self { provider }
    }

    pub async fn find_pairs(&self, token_address: Address) -> Result<Vec<PairInfo>> {
        let base_tokens = get_base_tokens();
        let mut pairs = Vec::new();

        // Check V2 factory
        if let Ok(v2_pairs) = self.find_v2_pairs(token_address, &base_tokens).await {
            pairs.extend(v2_pairs);
        }

        // Check V3 factory
        if let Ok(v3_pairs) = self.find_v3_pairs(token_address, &base_tokens).await {
            pairs.extend(v3_pairs);
        }

        // Filter pairs by liquidity (minimum $5000 USD)
        let token_str = format!("{:?}", token_address);
        let pairs_with_liquidity = self.filter_by_liquidity(pairs, &token_str).await;

        // Don't log "no pairs found" here - let the caller (streamer.rs) decide
        // This prevents misleading messages for Four.meme tokens that are on bonding curve

        Ok(pairs_with_liquidity)
    }
    
    /// Filter pairs by liquidity using DexScreener API
    async fn filter_by_liquidity(&self, pairs: Vec<PairInfo>, token_address: &str) -> Vec<PairInfo> {
        if pairs.is_empty() {
            return pairs;
        }
        
        // Query DexScreener for liquidity data
        let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", token_address);
        
        let liquidity_map = match reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        let mut map = std::collections::HashMap::new();
                        
                        if let Some(pairs_data) = data["pairs"].as_array() {
                            for pair in pairs_data {
                                if pair["chainId"] == "bsc" {
                                    if let (Some(pair_addr), Some(liquidity)) = (
                                        pair["pairAddress"].as_str(),
                                        pair["liquidity"]["usd"].as_f64()
                                    ) {
                                        let normalized_addr = pair_addr.to_lowercase();
                                        map.insert(normalized_addr, liquidity);
                                    }
                                }
                            }
                        }
                        
                        map
                    }
                    Err(e) => {
                        log::warn!("⚠️  Failed to parse DexScreener response: {}", e);
                        std::collections::HashMap::new()
                    }
                }
            }
            Err(e) => {
                log::warn!("⚠️  Failed to fetch liquidity from DexScreener: {}", e);
                std::collections::HashMap::new()
            }
        };
        
        // Filter pairs by liquidity
        let mut filtered_pairs = Vec::new();
        
        for pair in pairs {
            let pair_addr_str = format!("{:?}", pair.pair_address).to_lowercase();
            
            if let Some(&liquidity_usd) = liquidity_map.get(&pair_addr_str) {
                if liquidity_usd >= MIN_LIQUIDITY_USD {
                    let pool_type = if pair.is_v3 { "V3" } else { "V2" };
                    log::info!("✅ {} pair {} with {} has sufficient liquidity: ${:.0} USD", 
                        pool_type, &pair_addr_str[..10], pair.base_token_symbol, liquidity_usd);
                    filtered_pairs.push(pair);
                } else {
                    let pool_type = if pair.is_v3 { "V3" } else { "V2" };
                    log::warn!("❌ Filtered out {} pair {} with {} - insufficient liquidity: ${:.2} USD (min: ${:.0})", 
                        pool_type, &pair_addr_str[..10], pair.base_token_symbol, liquidity_usd, MIN_LIQUIDITY_USD);
                }
            } else {
                // If we can't get liquidity data, include the pair with a warning
                log::warn!("⚠️  Could not verify liquidity for pair {} with {}, including anyway", 
                    &pair_addr_str[..10], pair.base_token_symbol);
                filtered_pairs.push(pair);
            }
        }
        
        filtered_pairs
    }

    async fn find_v2_pairs(&self, token_address: Address, base_tokens: &[(String, Address)]) -> Result<Vec<PairInfo>> {
        let abi: Abi = serde_json::from_str(FACTORY_V2_ABI)?;
        let factory = Contract::new(get_factory_address(), abi, self.provider.clone());
        let mut pairs = Vec::new();

        log::debug!("🔍 Checking V2 pairs for token {:?} against {} base tokens", token_address, base_tokens.len());

        for (symbol, base_token_address) in base_tokens {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            match factory
                .method::<_, Address>("getPair", (token_address, *base_token_address))?
                .call()
                .await
            {
                Ok(pair_address) if !pair_address.is_zero() => {
                    log::info!("✅ Found V2 pair with {}: {:?}", symbol, pair_address);
                    pairs.push(PairInfo {
                        pair_address,
                        token: token_address,
                        base_token: *base_token_address,
                        base_token_symbol: symbol.clone(),
                        is_v3: false,
                    });
                }
                Ok(pair_address) => {
                    log::debug!("  ⚪ No V2 pair with {} (returned zero address: {:?})", symbol, pair_address);
                }
                Err(e) => {
                    log::error!("❌ Error checking V2 pair with {}: {:?}", symbol, e);
                }
            }
        }

        Ok(pairs)
    }

    async fn find_v3_pairs(&self, token_address: Address, base_tokens: &[(String, Address)]) -> Result<Vec<PairInfo>> {
        let abi: Abi = serde_json::from_str(FACTORY_V3_ABI)?;
        let factory = Contract::new(get_v3_factory_address(), abi, self.provider.clone());
        let mut pairs = Vec::new();

        log::debug!("🔍 Checking V3 pairs for token {:?} against {} base tokens", token_address, base_tokens.len());

        for (symbol, base_token_address) in base_tokens {
            // Try each fee tier
            for fee in V3_FEE_TIERS {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                match factory
                    .method::<_, Address>("getPool", (token_address, *base_token_address, fee))?
                    .call()
                    .await
                {
                    Ok(pool_address) if !pool_address.is_zero() => {
                        log::info!("✅ Found V3 pool with {} (fee: {}): {:?}", symbol, fee, pool_address);
                        pairs.push(PairInfo {
                            pair_address: pool_address,
                            token: token_address,
                            base_token: *base_token_address,
                            base_token_symbol: symbol.clone(),
                            is_v3: true,
                        });
                        break; // Found a pool for this base token, no need to check other fees
                    }
                    Ok(_) => {
                        log::debug!("  ⚪ No V3 pool with {} (fee: {})", symbol, fee);
                    }
                    Err(e) => {
                        log::error!("❌ Error checking V3 pool with {} (fee: {}): {:?}", symbol, fee, e);
                    }
                }
            }
        }

        Ok(pairs)
    }
}

