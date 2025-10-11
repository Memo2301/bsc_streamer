use crate::types::PriceStats;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
struct PriceHistory {
    prices: Vec<f64>,
    high: f64,
    low: f64,
    first_price: f64,
    last_price: Option<f64>,
    swap_count: u64,
}

pub struct PriceTracker {
    history: Arc<RwLock<HashMap<String, PriceHistory>>>,
}

impl PriceTracker {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_price(&self, token: &str, base_token: &str, price: f64) -> PriceStats {
        let key = format!("{}-{}", token, base_token);
        let mut history_map = self.history.write().await;

        let history = history_map.entry(key).or_insert_with(|| PriceHistory {
            prices: Vec::new(),
            high: price,
            low: price,
            first_price: price,
            last_price: None,
            swap_count: 0,
        });

        // Calculate changes
        let last_price = history.last_price;
        let price_change = last_price.map(|lp| price - lp);
        let price_change_percent = last_price.map(|lp| {
            if lp > 0.0 {
                ((price - lp) / lp) * 100.0
            } else {
                0.0
            }
        });

        // Update history
        history.prices.push(price);
        if history.prices.len() > 100 {
            history.prices.remove(0);
        }

        history.high = history.high.max(price);
        history.low = history.low.min(price);
        history.last_price = Some(price);
        history.swap_count += 1;

        PriceStats {
            current_price: price,
            last_price,
            price_change,
            price_change_percent,
            high: history.high,
            low: history.low,
            first_price: history.first_price,
            swap_count: history.swap_count as usize,
        }
    }

    pub fn get_trend_emoji(change_percent: Option<f64>) -> &'static str {
        match change_percent {
            None => "➡️",
            Some(p) if p > 5.0 => "🚀",
            Some(p) if p > 1.0 => "📈",
            Some(p) if p > 0.0 => "⬆️",
            Some(p) if p < -5.0 => "💥",
            Some(p) if p < -1.0 => "📉",
            Some(p) if p < 0.0 => "⬇️",
            _ => "➡️",
        }
    }
}

