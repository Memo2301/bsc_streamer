//! # BSC Token Streamer
//!
//! A high-performance, event-driven BSC token swap event streamer with automatic
//! Four.meme bonding curve to PancakeSwap migration detection.
//!
//! ## Features
//!
//! - Real-time swap event streaming
//! - Four.meme bonding curve support
//! - PancakeSwap V2 support
//! - Automatic migration detection
//! - Token discovery (find where a token is trading)
//! - Manual platform selection
//! - Price tracking and statistics
//!
//! ## Example
//!
//! ```rust,no_run
//! use bsc_streamer::{StreamerBuilder, Platform};
//! use ethers::providers::{Provider, Ws};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to BSC
//!     let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
//!     
//!     // Build and start streamer with auto-detection
//!     StreamerBuilder::new(provider)
//!         .token_address("0x...")
//!         .auto_detect() // Automatically find where token is trading
//!         .on_swap(|swap| {
//!             println!("Swap detected: {:?}", swap);
//!         })
//!         .start()
//!         .await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod core;
pub mod display;
pub mod multi_token_streamer;
pub mod types;

use anyhow::{anyhow, Result};
use ethers::providers::{Middleware, Provider, Ws};
use std::sync::Arc;

pub use multi_token_streamer::MultiTokenStreamer;
pub use types::{MigrationEvent, Platform, SwapEvent, TradeType};

use crate::core::streamer::SwapStreamer;

/// Builder for configuring and starting a token swap event streamer
pub struct StreamerBuilder<M> {
    provider: Arc<M>,
    token_address: Option<String>,
    platform: Option<Platform>,
    auto_detect: bool,
}

impl StreamerBuilder<Provider<Ws>> {
    /// Create a new streamer builder with a WebSocket URL
    ///
    /// # Example
    /// ```rust,no_run
    /// use bsc_streamer::StreamerBuilder;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let streamer = StreamerBuilder::from_wss("wss://bsc.publicnode.com").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn from_wss(wss_url: &str) -> Result<Self> {
        let provider = Provider::<Ws>::connect(wss_url).await?;
        Ok(Self::new(Arc::new(provider)))
    }
}

impl<M: Middleware + 'static> StreamerBuilder<M> {
    /// Create a new streamer builder with an existing provider
    ///
    /// # Example
    /// ```rust,no_run
    /// use bsc_streamer::StreamerBuilder;
    /// use ethers::providers::{Provider, Ws};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
    ///     let streamer = StreamerBuilder::new(Arc::new(provider));
    ///     Ok(())
    /// }
    /// ```
    pub fn new(provider: Arc<M>) -> Self {
        Self {
            provider,
            token_address: None,
            platform: None,
            auto_detect: false,
        }
    }

    /// Set the token address to monitor
    pub fn token_address(mut self, address: &str) -> Self {
        self.token_address = Some(address.to_string());
        self
    }

    /// Manually specify the platform where the token is trading
    ///
    /// # Example
    /// ```rust,no_run
    /// use bsc_streamer::{StreamerBuilder, Platform};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// StreamerBuilder::from_wss("wss://bsc.publicnode.com")
    ///     .await?
    ///     .token_address("0x...")
    ///     .platform(Platform::FourMemeBondingCurve) // Manually set platform
    ///     .on_swap(|swap| { /* ... */ })
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self.auto_detect = false;
        self
    }

    /// Enable automatic platform detection
    ///
    /// The streamer will check if the token is on Four.meme bonding curve,
    /// then search for PancakeSwap pairs, and automatically handle migration.
    pub fn auto_detect(mut self) -> Self {
        self.auto_detect = true;
        self.platform = None;
        self
    }

    /// Start the streamer with a callback for swap events
    ///
    /// # Example
    /// ```rust,no_run
    /// use bsc_streamer::StreamerBuilder;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// StreamerBuilder::from_wss("wss://bsc.publicnode.com")
    ///     .await?
    ///     .token_address("0x...")
    ///     .auto_detect()
    ///     .on_swap(|swap| {
    ///         println!("Trade: {} {} for {} {}",
    ///             swap.trade_type.as_str(),
    ///             swap.token.amount,
    ///             swap.base_token.amount,
    ///             swap.base_token.symbol
    ///         );
    ///     })
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_swap<F>(self, callback: F) -> StreamerRunner<M, F, fn(MigrationEvent)>
    where
        F: Fn(SwapEvent) + Send + Sync + 'static,
    {
        StreamerRunner {
            builder: self,
            swap_callback: callback,
            migration_callback: None,
        }
    }
}

/// Runner that holds the callbacks and starts the streamer
pub struct StreamerRunner<M, F, G> {
    builder: StreamerBuilder<M>,
    swap_callback: F,
    migration_callback: Option<G>,
}

impl<M, F, G> StreamerRunner<M, F, G>
where
    M: Middleware + 'static,
    F: Fn(SwapEvent) + Send + Sync + 'static,
    G: Fn(MigrationEvent) + Send + Sync + 'static,
{
    /// Set a callback for migration events
    ///
    /// This callback will be triggered when a token migrates from Four.meme bonding curve to PancakeSwap
    ///
    /// # Example
    /// ```rust,no_run
    /// use bsc_streamer::StreamerBuilder;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// StreamerBuilder::from_wss("wss://bsc.publicnode.com")
    ///     .await?
    ///     .token_address("0x...")
    ///     .auto_detect()
    ///     .on_swap(|swap| {
    ///         println!("Swap: {:?}", swap);
    ///     })
    ///     .on_migration(|migration| {
    ///         println!("🎉 Migration detected!");
    ///         println!("From: {}", migration.from_platform.as_str());
    ///         println!("To: {}", migration.to_platform.as_str());
    ///         println!("Pairs: {}", migration.pair_count);
    ///     })
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_migration<H>(self, callback: H) -> StreamerRunner<M, F, H>
    where
        H: Fn(MigrationEvent) + Send + Sync + 'static,
    {
        StreamerRunner {
            builder: self.builder,
            swap_callback: self.swap_callback,
            migration_callback: Some(callback),
        }
    }

    /// Start streaming swap events
    pub async fn start(self) -> Result<()> {
        let token_address = self
            .builder
            .token_address
            .ok_or_else(|| anyhow!("Token address is required"))?;

        let mut streamer = SwapStreamer::new(self.builder.provider);

        if self.builder.auto_detect {
            // Auto-detect mode: Let streamer figure out where token is
            streamer.start_with_migration_callback(
                &token_address, 
                self.swap_callback,
                self.migration_callback,
            ).await?;
        } else if let Some(platform) = self.builder.platform {
            // Manual platform mode
            match platform {
                Platform::FourMemeBondingCurve => {
                    // Start bonding curve monitoring with migration detection
                    streamer.start_with_migration_callback(
                        &token_address,
                        self.swap_callback,
                        self.migration_callback,
                    ).await?;
                }
                Platform::PancakeSwap => {
                    // Start DEX monitoring only
                    streamer.start_with_migration_callback(
                        &token_address,
                        self.swap_callback,
                        self.migration_callback,
                    ).await?;
                }
            }
        } else {
            return Err(anyhow!("Must either enable auto_detect() or specify platform()"));
        }

        Ok(())
    }
}

/// Find where a token is currently trading
///
/// Returns information about where the token can be found (bonding curve, DEX pairs, etc.)
///
/// # Example
/// ```rust,no_run
/// use bsc_streamer::find_token_location;
/// use ethers::providers::{Provider, Ws};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = Provider::<Ws>::connect("wss://bsc.publicnode.com").await?;
/// let location = find_token_location(&provider, "0x...").await?;
/// println!("Token found on: {:?}", location);
/// # Ok(())
/// # }
/// ```
pub async fn find_token_location<M: Middleware + Clone + 'static>(
    provider: Arc<M>,
    token_address: &str,
) -> Result<TokenLocation> {
    use crate::core::pair_finder::PairFinder;
    use ethers::types::Address;
    use std::str::FromStr;

    let token_address = Address::from_str(token_address)?;

    // Check bonding curve first
    let streamer = SwapStreamer::new(provider.clone());
    let on_bonding_curve = streamer.check_bonding_curve_public(&token_address).await?;

    // Check for DEX pairs
    let pair_finder = PairFinder::new(provider);
    let pairs = pair_finder.find_pairs(token_address).await.unwrap_or_default();

    Ok(TokenLocation {
        on_bonding_curve,
        dex_pairs: pairs.len(),
        platforms: if on_bonding_curve {
            vec![Platform::FourMemeBondingCurve]
        } else if !pairs.is_empty() {
            vec![Platform::PancakeSwap]
        } else {
            vec![]
        },
    })
}

/// Information about where a token is currently trading
#[derive(Debug, Clone)]
pub struct TokenLocation {
    /// Whether the token is on Four.meme bonding curve
    pub on_bonding_curve: bool,
    /// Number of DEX pairs found
    pub dex_pairs: usize,
    /// Platforms where the token is available
    pub platforms: Vec<Platform>,
}

