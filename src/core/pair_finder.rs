use anyhow::Result;
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::Middleware,
    types::Address,
};
use std::sync::Arc;

use crate::config::{get_base_tokens, get_factory_address};
use crate::types::PairInfo;

const FACTORY_ABI: &str = r#"[
    {"constant":true,"inputs":[{"name":"tokenA","type":"address"},{"name":"tokenB","type":"address"}],"name":"getPair","outputs":[{"name":"pair","type":"address"}],"type":"function"}
]"#;

pub struct PairFinder<M> {
    provider: Arc<M>,
}

impl<M: Middleware + 'static> PairFinder<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self { provider }
    }

    pub async fn find_pairs(&self, token_address: Address) -> Result<Vec<PairInfo>> {
        println!("🔍 Finding pairs for token...\n");

        let factory_address = get_factory_address();
        let abi: Abi = serde_json::from_str(FACTORY_ABI)?;
        let factory = Contract::new(factory_address, abi, self.provider.clone());

        let base_tokens = get_base_tokens();
        let mut pairs = Vec::new();

        for (symbol, base_token_address) in base_tokens {
            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            match factory
                .method::<_, Address>("getPair", (token_address, base_token_address))?
                .call()
                .await
            {
                Ok(pair_address) if !pair_address.is_zero() => {
                    println!("  ✅ Found pair with {}: {:?}", symbol, pair_address);
                    pairs.push(PairInfo {
                        pair_address,
                        token: token_address,
                        base_token: base_token_address,
                        base_token_symbol: symbol,
                    });
                }
                Ok(_) => {
                    println!("  ⚪ No {} pair", symbol);
                }
                Err(e) => {
                    println!("  ⚠️  Error checking {} pair: {}", symbol, e);
                }
            }
        }

        if pairs.is_empty() {
            println!("  ⚠️  No pairs found for this token");
            println!("  💡 This token might not have liquidity on PancakeSwap");
            println!("  💡 Try a different token or check if it exists on other DEXs\n");
        }

        Ok(pairs)
    }
}

