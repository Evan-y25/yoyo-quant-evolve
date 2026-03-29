//! Custom trading tools for yoyo.
//!
//! These tools give yoyo eyes on the market — real-time prices, search, and overviews
//! from CoinGecko (crypto) and Yahoo Finance (stocks).

pub mod crypto;
pub mod market_overview;
pub mod search_symbol;

pub use crypto::GetPriceTool;
pub use market_overview::GetMarketOverviewTool;
pub use search_symbol::SearchSymbolTool;

use yoagent::types::AgentTool;

/// Get all custom trading tools.
pub fn trading_tools() -> Vec<Box<dyn AgentTool>> {
    vec![
        Box::new(GetPriceTool::new()),
        Box::new(SearchSymbolTool::new()),
        Box::new(GetMarketOverviewTool::new()),
    ]
}
