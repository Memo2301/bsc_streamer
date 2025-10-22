use anyhow::{anyhow, Result};
use ethers::providers::Middleware;
use ethers::types::Address;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::core::streamer::SwapStreamer;
use crate::types::{MigrationEvent, SwapEvent};

/// Information about a monitored token
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: Address,
    pub cancellation_token: CancellationToken,
}

/// Multi-token streamer that can dynamically add/remove tokens
pub struct MultiTokenStreamer<M> {
    provider: Arc<M>,
    tokens: Arc<RwLock<HashMap<Address, CancellationToken>>>,
}

impl<M> MultiTokenStreamer<M>
where
    M: Middleware + 'static,
{
    /// Create a new multi-token streamer
    pub fn new(provider: Arc<M>) -> Self {
        Self {
            provider,
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a token to monitor
    ///
    /// # Arguments
    /// * `token_address` - The token address to monitor
    /// * `swap_callback` - Callback for swap events
    /// * `migration_callback` - Optional callback for migration events
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsc_streamer::MultiTokenStreamer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let streamer = MultiTokenStreamer::new(provider);
    /// 
    /// streamer.add_token(
    ///     "0x...",
    ///     |swap| println!("Swap: {:?}", swap),
    ///     Some(|migration| println!("Migration: {:?}", migration))
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_token<F, G>(
        &self,
        token_address: &str,
        swap_callback: F,
        migration_callback: Option<G>,
    ) -> Result<()>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
        G: Fn(MigrationEvent) + Send + Sync + 'static,
        M::Provider: ethers::providers::PubsubClient,
    {
        let address = Address::from_str(token_address)?;

        // Check if already monitoring
        {
            let tokens = self.tokens.read().await;
            if tokens.contains_key(&address) {
                return Err(anyhow!("Token {:?} is already being monitored", address));
            }
        }

        println!("\n➕ Adding token to streamer: {:?}", address);

        // Create cancellation token for this token's monitoring
        let cancel_token = CancellationToken::new();

        // Add to tokens map
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(address, cancel_token.clone());
        }

        // Start monitoring in a separate task
        let provider_clone = self.provider.clone();
        let cancel_token_clone = cancel_token.clone();
        let tokens_clone = self.tokens.clone();

        tokio::spawn(async move {
            let mut streamer = SwapStreamer::new(provider_clone);
            // Format address as hex string with 0x prefix
            let address_str = format!("{:#x}", address);

            // Create a select between the streamer and cancellation
            tokio::select! {
                result = streamer.start_with_migration_callback(
                    &address_str,
                    swap_callback,
                    migration_callback,
                ) => {
                    if let Err(e) = result {
                        eprintln!("❌ Error monitoring token {:?}: {}", address, e);
                    }
                }
                _ = cancel_token_clone.cancelled() => {
                    println!("🛑 Stopped monitoring token: {:?}", address);
                }
            }

            // Clean up from tokens map
            let mut tokens = tokens_clone.write().await;
            tokens.remove(&address);
        });

        println!("✅ Token added successfully\n");

        Ok(())
    }

    /// Remove a token from monitoring
    ///
    /// # Arguments
    /// * `token_address` - The token address to stop monitoring
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsc_streamer::MultiTokenStreamer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let streamer = MultiTokenStreamer::new(provider);
    /// streamer.remove_token("0x...").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_token(&self, token_address: &str) -> Result<()> {
        let address = Address::from_str(token_address)?;

        let cancel_token = {
            let tokens = self.tokens.read().await;
            tokens.get(&address).cloned()
        };

        match cancel_token {
            Some(token) => {
                println!("\n➖ Removing token from streamer: {:?}", address);
                token.cancel();
                println!("✅ Token removal initiated\n");
                Ok(())
            }
            None => Err(anyhow!("Token {:?} is not being monitored", address)),
        }
    }

    /// Get list of currently monitored tokens
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsc_streamer::MultiTokenStreamer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let streamer = MultiTokenStreamer::new(provider);
    /// let tokens = streamer.list_tokens().await;
    /// println!("Monitoring {} tokens", tokens.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_tokens(&self) -> Vec<Address> {
        let tokens = self.tokens.read().await;
        tokens.keys().copied().collect()
    }

    /// Get the number of tokens currently being monitored
    pub async fn token_count(&self) -> usize {
        let tokens = self.tokens.read().await;
        tokens.len()
    }

    /// Check if a specific token is being monitored
    pub async fn is_monitoring(&self, token_address: &str) -> Result<bool> {
        let address = Address::from_str(token_address)?;
        let tokens = self.tokens.read().await;
        Ok(tokens.contains_key(&address))
    }

    /// Stop monitoring all tokens
    pub async fn stop_all(&self) {
        println!("\n🛑 Stopping all token monitoring...");
        let tokens = self.tokens.read().await;
        for (address, token) in tokens.iter() {
            println!("  Stopping: {:?}", address);
            token.cancel();
        }
        println!("✅ All tokens stopped\n");
    }
}

impl<M> Clone for MultiTokenStreamer<M> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            tokens: self.tokens.clone(),
        }
    }
}

