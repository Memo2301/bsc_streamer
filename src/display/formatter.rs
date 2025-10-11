use colored::*;

use crate::core::price_tracker::PriceTracker;
use crate::types::{SwapEvent, TradeType};

pub struct SwapFormatter {
    price_tracker: PriceTracker,
}

impl SwapFormatter {
    pub fn new() -> Self {
        Self {
            price_tracker: PriceTracker::new(),
        }
    }

    pub fn display(&self, swap: &SwapEvent) {
        // Update price tracking
        let price_stats = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.price_tracker
                    .update_price(
                        &format!("{:?}", swap.token.address),
                        &swap.price.base_token,
                        swap.price.value,
                    )
                    .await
            })
        });

        // Get emoji and trend
        let emoji = match swap.trade_type {
            TradeType::Buy => "🟢",
            TradeType::Sell => "🔴",
        };
        let trend = PriceTracker::get_trend_emoji(price_stats.price_change_percent);

        // Display trade info
        println!(
            "{} {} {} {} [{}]",
            emoji,
            swap.trade_type.as_str(),
            swap.token.symbol.bright_white().bold(),
            trend,
            swap.platform.as_str().cyan()
        );

        println!(
            "   Amount: {} {}",
            format!("{:.6}", swap.token.amount.parse::<f64>().unwrap_or(0.0)).bright_yellow(),
            swap.token.symbol
        );

        println!(
            "   For: {} {}",
            format!("{:.6}", swap.base_token.amount.parse::<f64>().unwrap_or(0.0)).bright_yellow(),
            swap.base_token.symbol
        );

        println!("   Price: {}", swap.price.display.bright_cyan());

        // Display price change if available
        if let Some(change_percent) = price_stats.price_change_percent {
            let change_symbol = if change_percent >= 0.0 { "+" } else { "" };
            let change_color = if change_percent >= 0.0 {
                "🟢".green()
            } else {
                "🔴".red()
            };

            if let Some(change) = price_stats.price_change {
                println!(
                    "   Change: {} {}{:.2}% ({}{:.4e} {})",
                    change_color,
                    change_symbol,
                    change_percent,
                    change_symbol,
                    change,
                    swap.price.base_token
                );
            }
        }

        // Display session stats
        if price_stats.swap_count > 1 {
            let total_change_percent =
                ((price_stats.current_price - price_stats.first_price) / price_stats.first_price) * 100.0;
            let change_symbol = if total_change_percent >= 0.0 { "+" } else { "" };

            println!(
                "   Session: {}{:.2}% | High: {:.12} | Low: {:.12} | Swaps: {}",
                change_symbol,
                total_change_percent,
                price_stats.high,
                price_stats.low,
                price_stats.swap_count
            );
        }

        // Display pair or bonding curve address
        if let Some(bc_addr) = swap.bonding_curve_address {
            println!("   Bonding Curve: {:?}", bc_addr);
        } else if let Some(pair_addr) = swap.pair_address {
            println!("   Pair: {:?}", pair_addr);
        }

        println!(
            "   Tx: https://bscscan.com/tx/{:?}",
            swap.transaction_hash
        );

        if let Some(ref timestamp) = swap.timestamp {
            println!("   Time: {}", timestamp);
        }

        println!("{}", "─".repeat(80).bright_black());
    }
}

