use anyhow::Result;
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::Middleware,
    types::Address,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const ERC20_ABI: &str = r#"[
    {"constant":true,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":true,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":true,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"}
]"#;

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

pub struct TokenInfoCache<M> {
    provider: Arc<M>,
    cache: Arc<RwLock<HashMap<Address, TokenMetadata>>>,
}

impl<M: Middleware + 'static> TokenInfoCache<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self {
            provider,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_token_info(&self, address: Address) -> Result<TokenMetadata> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(&address) {
                return Ok(info.clone());
            }
        }

        // Fetch from contract
        let abi: Abi = serde_json::from_str(ERC20_ABI)?;
        let contract = Contract::new(address, abi, self.provider.clone());

        let name: String = contract
            .method::<_, String>("name", ())?
            .call()
            .await
            .unwrap_or_else(|_| "Unknown".to_string());

        let symbol: String = contract
            .method::<_, String>("symbol", ())?
            .call()
            .await
            .unwrap_or_else(|_| "UNKNOWN".to_string());

        let decimals: u8 = contract
            .method::<_, u8>("decimals", ())?
            .call()
            .await
            .unwrap_or(18);

        let metadata = TokenMetadata {
            name,
            symbol,
            decimals,
        };

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(address, metadata.clone());
        }

        Ok(metadata)
    }
}

