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
        println!("🔍 Finding pairs for token...\n");

        let base_tokens = get_base_tokens();
        let mut pairs = Vec::new();

        // Check V2 factory
        println!("  Checking PancakeSwap V2 Factory...");
        if let Ok(v2_pairs) = self.find_v2_pairs(token_address, &base_tokens).await {
            pairs.extend(v2_pairs);
        }

        // Check V3 factory
        println!("  Checking PancakeSwap V3 Factory...");
        if let Ok(v3_pairs) = self.find_v3_pairs(token_address, &base_tokens).await {
            pairs.extend(v3_pairs);
        }

        // Only show "no pairs" message if we checked both factories and found nothing
        if pairs.is_empty() {
            println!("  ⚠️  No pairs found for this token in V2 or V3");
            println!("  💡 This token might not have liquidity on PancakeSwap");
            println!("  💡 Try a different token or check if it exists on other DEXs\n");
        }

        Ok(pairs)
    }

    async fn find_v2_pairs(&self, token_address: Address, base_tokens: &[(String, Address)]) -> Result<Vec<PairInfo>> {
        let abi: Abi = serde_json::from_str(FACTORY_V2_ABI)?;
        let factory = Contract::new(get_factory_address(), abi, self.provider.clone());
        let mut pairs = Vec::new();

        for (symbol, base_token_address) in base_tokens {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            match factory
                .method::<_, Address>("getPair", (token_address, *base_token_address))?
                .call()
                .await
            {
                Ok(pair_address) if !pair_address.is_zero() => {
                    println!("  ✅ Found V2 {} pair: {:?}", symbol, pair_address);
                    pairs.push(PairInfo {
                        pair_address,
                        token: token_address,
                        base_token: *base_token_address,
                        base_token_symbol: symbol.clone(),
                        is_v3: false,
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    println!("  ⚠️  Error checking V2 {} pair: {}", symbol, e);
                }
            }
        }

        Ok(pairs)
    }

    async fn find_v3_pairs(&self, token_address: Address, base_tokens: &[(String, Address)]) -> Result<Vec<PairInfo>> {
        let abi: Abi = serde_json::from_str(FACTORY_V3_ABI)?;
        let factory = Contract::new(get_v3_factory_address(), abi, self.provider.clone());
        let mut pairs = Vec::new();

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
                        println!("  ✅ Found V3 {} pool (fee {}): {:?}", symbol, fee, pool_address);
                        pairs.push(PairInfo {
                            pair_address: pool_address,
                            token: token_address,
                            base_token: *base_token_address,
                            base_token_symbol: symbol.clone(),
                            is_v3: true,
                        });
                        break; // Found a pool for this base token, no need to check other fees
                    }
                    Ok(_) => {}
                    Err(e) => {
                        println!("  ⚠️  Error checking V3 {} pool (fee {}): {}", symbol, fee, e);
                    }
                }
            }
        }

        Ok(pairs)
    }
}

