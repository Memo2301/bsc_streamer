use anyhow::{anyhow, Result};
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::Middleware,
    types::{Address, Log, U256},
    utils::format_units,
};
use std::sync::Arc;

use crate::core::token_info::TokenInfoCache;
use crate::types::{PairInfo, Platform, PriceInfo, SwapEvent, TokenInfo, TradeType};

const PAIR_ABI: &str = r#"[
    {"constant":true,"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},
    {"constant":true,"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"},
    {"anonymous":false,"inputs":[{"indexed":true,"name":"sender","type":"address"},{"indexed":false,"name":"amount0In","type":"uint256"},{"indexed":false,"name":"amount1In","type":"uint256"},{"indexed":false,"name":"amount0Out","type":"uint256"},{"indexed":false,"name":"amount1Out","type":"uint256"},{"indexed":true,"name":"to","type":"address"}],"name":"Swap","type":"event"}
]"#;

pub struct SwapParser<M> {
    pub provider: Arc<M>,
    pub token_cache: TokenInfoCache<M>,
}

impl<M: Middleware + 'static> SwapParser<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self {
            token_cache: TokenInfoCache::new(provider.clone()),
            provider,
        }
    }

    pub async fn parse_swap_event(
        &self,
        log: &Log,
        pair_info: &PairInfo,
    ) -> Result<SwapEvent> {
        let abi: Abi = serde_json::from_str(PAIR_ABI)?;
        let contract = Contract::new(pair_info.pair_address, abi.clone(), self.provider.clone());

        // Get token addresses
        let token0: Address = contract.method("token0", ())?.call().await?;
        let token1: Address = contract.method("token1", ())?.call().await?;

        // Get token info
        let token0_info = self.token_cache.get_token_info(token0).await?;
        let token1_info = self.token_cache.get_token_info(token1).await?;

        // Parse event
        let event = abi.events().find(|e| e.name == "Swap")
            .ok_or_else(|| anyhow!("Swap event not found in ABI"))?;
        let parsed = event.parse_log(log.clone().into())?;

        // Safely extract parameters with proper error handling
        let amount0_in: U256 = parsed.params.get(0)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or_else(|| anyhow!("Failed to parse amount0In"))?;
        let amount1_in: U256 = parsed.params.get(1)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or_else(|| anyhow!("Failed to parse amount1In"))?;
        let amount0_out: U256 = parsed.params.get(2)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or_else(|| anyhow!("Failed to parse amount0Out"))?;
        let amount1_out: U256 = parsed.params.get(3)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or_else(|| anyhow!("Failed to parse amount1Out"))?;
        let to: Address = parsed.params.get(4)
            .and_then(|p| p.value.clone().into_address())
            .ok_or_else(|| anyhow!("Failed to parse to address"))?;
        let sender: Address = Address::from(log.topics[1]);

        // Determine trade type and amounts
        let is_token0_target = token0 == pair_info.token;
        let (trade_type, token_amount, base_amount, token_decimals, base_decimals) =
            if is_token0_target {
                if amount0_out > U256::zero() {
                    (
                        TradeType::Buy,
                        amount0_out,
                        amount1_in,
                        token0_info.decimals,
                        token1_info.decimals,
                    )
                } else {
                    (
                        TradeType::Sell,
                        amount0_in,
                        amount1_out,
                        token0_info.decimals,
                        token1_info.decimals,
                    )
                }
            } else {
                if amount1_out > U256::zero() {
                    (
                        TradeType::Buy,
                        amount1_out,
                        amount0_in,
                        token1_info.decimals,
                        token0_info.decimals,
                    )
                } else {
                    (
                        TradeType::Sell,
                        amount1_in,
                        amount0_out,
                        token1_info.decimals,
                        token0_info.decimals,
                    )
                }
            };

        let token_amount_str = format_units(token_amount, token_decimals as u32)?;
        let base_amount_str = format_units(base_amount, base_decimals as u32)?;

        // Calculate price
        let token_amount_f64: f64 = token_amount_str.parse().unwrap_or(0.0);
        let base_amount_f64: f64 = base_amount_str.parse().unwrap_or(0.0);
        let price = if token_amount_f64 > 0.0 {
            base_amount_f64 / token_amount_f64
        } else {
            0.0
        };

        // Get block info
        let block = self.provider.get_block(log.block_number.unwrap()).await?;
        let timestamp = block.and_then(|b| {
            b.timestamp
                .as_u64()
                .checked_mul(1000)
                .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
                .map(|dt| dt.to_rfc3339())
        });

        Ok(SwapEvent {
            transaction_hash: log.transaction_hash.unwrap(),
            block_number: log.block_number.unwrap().as_u64(),
            timestamp,
            platform: Platform::PancakeSwap,
            trade_type,
            token: TokenInfo {
                address: pair_info.token,
                symbol: if is_token0_target {
                    token0_info.symbol
                } else {
                    token1_info.symbol
                },
                amount: token_amount_str,
                decimals: token_decimals,
            },
            base_token: TokenInfo {
                address: pair_info.base_token,
                symbol: pair_info.base_token_symbol.clone(),
                amount: base_amount_str,
                decimals: base_decimals,
            },
            price: PriceInfo {
                value: price,
                display: format!("{:.12} {}", price, pair_info.base_token_symbol),
                base_token: pair_info.base_token_symbol.clone(),
            },
            sender,
            recipient: to,
            pair_address: Some(pair_info.pair_address),
            bonding_curve_address: None,
        })
    }

    pub async fn parse_bonding_curve_event(
        &self,
        log: &Log,
        token_address: Address,
        bonding_curve_address: Address,
    ) -> Result<Option<SwapEvent>> {
        // Parse Transfer event
        let from = Address::from(log.topics[1]);
        let to = Address::from(log.topics[2]);
        let value = U256::from_big_endian(&log.data);

        // Determine trade type
        let (trade_type, token_amount) = if from == bonding_curve_address {
            (TradeType::Buy, value)
        } else if to == bonding_curve_address {
            (TradeType::Sell, value)
        } else {
            return Ok(None); // Not a bonding curve trade
        };

        // Get token info
        let token_info = self.token_cache.get_token_info(token_address).await?;

        // Get transaction to extract BNB amount
        let bnb_amount = if trade_type == TradeType::Buy {
            // For buys, check tx.value first
            let tx = self
                .provider
                .get_transaction(log.transaction_hash.unwrap())
                .await?;
            let tx_value = tx.map(|t| t.value).unwrap_or_default();
            
            // If tx.value is 0, the bonding curve might use a different mechanism
            // (e.g., WBNB deposit, pre-approved balance, etc.)
            // In that case, also check the receipt for the BNB amount
            if tx_value == U256::zero() {
                if let Some(receipt) = self
                    .provider
                    .get_transaction_receipt(log.transaction_hash.unwrap())
                    .await?
                {
                    let mut found_bnb = U256::zero();
                    
                    // Look for bonding curve events with BNB amount
                    for tx_log in &receipt.logs {
                        if tx_log.address == bonding_curve_address {
                            if tx_log.data.len() >= 160 {
                                let bnb_amount = U256::from_big_endian(&tx_log.data[128..160]);
                                if bnb_amount > U256::zero() && bnb_amount < U256::from(1000) * U256::from(10u64.pow(18)) {
                                    found_bnb = bnb_amount;
                                    break;
                                }
                            }
                            
                            if found_bnb == U256::zero() && tx_log.data.len() >= 96 {
                                let potential_amount = U256::from_big_endian(&tx_log.data[64..96]);
                                if potential_amount > U256::zero() && potential_amount < U256::from(1000) * U256::from(10u64.pow(18)) {
                                    found_bnb = potential_amount;
                                    break;
                                }
                            }
                        }
                    }
                    
                    found_bnb
                } else {
                    U256::zero()
                }
            } else {
                tx_value
            }
        } else {
            // For sells, check the transaction receipt for bonding curve events
            // The bonding curve contract should emit events with the BNB amount
            if let Some(receipt) = self
                .provider
                .get_transaction_receipt(log.transaction_hash.unwrap())
                .await?
            {
                // Look through all logs for events from the bonding curve
                // Common event signatures to look for:
                // - Swap, Trade, Sell events that might contain BNB amount
                // - WBNB Transfer events (bonding curve might use WBNB)
                
                let mut found_bnb = U256::zero();
                
                // Look for any logs from the bonding curve contract
                for tx_log in &receipt.logs {
                    if tx_log.address == bonding_curve_address {
                        // Bonding curve events typically have this structure:
                        // - data[0:32] = token address
                        // - data[32:64] = user address  
                        // - data[64:96] = some value (fee/dust)
                        // - data[96:128] = token amount
                        // - data[128:160] = BNB amount ← THIS IS WHAT WE NEED
                        
                        // Try to extract BNB amount from offset 128 (5th field)
                        if tx_log.data.len() >= 160 {
                            let bnb_amount = U256::from_big_endian(&tx_log.data[128..160]);
                            if bnb_amount > U256::zero() && bnb_amount < U256::from(1000) * U256::from(10u64.pow(18)) {
                                // Reasonable BNB amount (less than 1000 BNB)
                                found_bnb = bnb_amount;
                                break;
                            }
                        }
                        
                        // Fallback: try other offsets if offset 128 didn't work
                        if found_bnb == U256::zero() && tx_log.data.len() >= 96 {
                            // Try offset 64 (3rd field)
                            let potential_amount = U256::from_big_endian(&tx_log.data[64..96]);
                            if potential_amount > U256::zero() && potential_amount < U256::from(1000) * U256::from(10u64.pow(18)) {
                                found_bnb = potential_amount;
                                break;
                            }
                        }
                    }
                }
                
                found_bnb
            } else {
                U256::zero()
            }
        };

        let token_amount_str = format_units(token_amount, token_info.decimals as u32)?;
        let bnb_amount_str = format_units(bnb_amount, 18u32)?;

        // Calculate price
        let token_amount_f64: f64 = token_amount_str.parse().unwrap_or(0.0);
        let bnb_amount_f64: f64 = bnb_amount_str.parse().unwrap_or(0.0);
        let price = if token_amount_f64 > 0.0 {
            bnb_amount_f64 / token_amount_f64
        } else {
            0.0
        };

        // Get block info
        let block = self.provider.get_block(log.block_number.unwrap()).await?;
        let timestamp = block.and_then(|b| {
            b.timestamp
                .as_u64()
                .checked_mul(1000)
                .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
                .map(|dt| dt.to_rfc3339())
        });

        Ok(Some(SwapEvent {
            transaction_hash: log.transaction_hash.unwrap(),
            block_number: log.block_number.unwrap().as_u64(),
            timestamp,
            platform: Platform::FourMemeBondingCurve,
            trade_type,
            token: TokenInfo {
                address: token_address,
                symbol: token_info.symbol,
                amount: token_amount_str,
                decimals: token_info.decimals,
            },
            base_token: TokenInfo {
                address: Address::zero(), // Native BNB
                symbol: "BNB".to_string(),
                amount: bnb_amount_str,
                decimals: 18,
            },
            price: PriceInfo {
                value: price,
                display: format!("{:.12} BNB", price),
                base_token: "BNB".to_string(),
            },
            sender: from,
            recipient: to,
            pair_address: None,
            bonding_curve_address: Some(bonding_curve_address),
        }))
    }
}

