use ethers::types::{Address, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    PancakeSwap,
    FourMemeBondingCurve,
}

impl Platform {
    pub fn as_str(&self) -> &str {
        match self {
            Platform::PancakeSwap => "PancakeSwap V2",
            Platform::FourMemeBondingCurve => "Four.meme Bonding Curve",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    Buy,
    Sell,
}

impl TradeType {
    pub fn as_str(&self) -> &str {
        match self {
            TradeType::Buy => "BUY",
            TradeType::Sell => "SELL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: Address,
    pub symbol: String,
    pub amount: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    pub value: f64,
    pub display: String,
    pub base_token: String,
}

#[derive(Debug, Clone)]
pub struct PairInfo {
    pub pair_address: Address,
    pub token: Address,
    pub base_token: Address,
    pub base_token_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingCurveInfo {
    pub address: Address,
    pub token: Address,
}

#[derive(Debug, Clone)]
pub struct PriceStats {
    pub current_price: f64,
    pub last_price: Option<f64>,
    pub price_change: Option<f64>,
    pub price_change_percent: Option<f64>,
    pub high: f64,
    pub low: f64,
    pub first_price: f64,
    pub swap_count: usize,
}

/// Event emitted when a token migrates from bonding curve to DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEvent {
    /// Token address that migrated
    pub token_address: Address,
    /// Source platform (usually FourMemeBondingCurve)
    pub from_platform: Platform,
    /// Destination platform (usually PancakeSwap)
    pub to_platform: Platform,
    /// Transaction hash of the migration (PairCreated transaction)
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

impl MigrationEvent {
    pub fn as_message(&self) -> String {
        format!(
            "🎉 MIGRATION DETECTED! Token migrated from {} to {} at block {}. Found {} DEX pair(s).",
            self.from_platform.as_str(),
            self.to_platform.as_str(),
            self.block_number,
            self.pair_count
        )
    }
}
