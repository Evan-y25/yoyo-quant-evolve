//! Custom trading tools for yoyo.
//!
//! These tools give yoyo eyes on the market — real-time prices, search, overviews,
//! news, and analysis from CoinGecko (crypto) and Yahoo Finance (stocks).

pub mod alerts;
pub mod crypto;
pub mod format;
pub mod http;
pub mod indicators;
pub mod market_overview;
pub mod news;
pub mod portfolio;
pub mod price_history;
pub mod search_symbol;
pub mod watchlist;

pub use crypto::fetch_live_price;
pub use crypto::GetPriceTool;
pub use market_overview::GetMarketOverviewTool;
pub use news::GetNewsTool;
pub use price_history::GetPriceHistoryTool;
pub use price_history::{compute_signal_counts, SignalCounts};
pub use search_symbol::SearchSymbolTool;

use yoagent::types::AgentTool;

/// Get all custom trading tools.
pub fn trading_tools() -> Vec<Box<dyn AgentTool>> {
    vec![
        Box::new(GetPriceTool::new()),
        Box::new(SearchSymbolTool::new()),
        Box::new(GetMarketOverviewTool::new()),
        Box::new(GetPriceHistoryTool::new()),
        Box::new(GetNewsTool::new()),
    ]
}
