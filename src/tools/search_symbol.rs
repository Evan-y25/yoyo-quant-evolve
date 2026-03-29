//! search_symbol tool — search for cryptocurrency or stock symbols by name.
//!
//! Uses CoinGecko's search endpoint for crypto, and a local mapping for common stocks.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use yoagent::types::*;

use super::http::{create_client, fetch_json_with_retry};

const COINGECKO_BASE: &str = "https://api.coingecko.com/api/v3";

pub struct SearchSymbolTool {
    client: Client,
}

impl SearchSymbolTool {
    pub fn new() -> Self {
        Self {
            client: create_client(),
        }
    }
}

#[async_trait]
impl AgentTool for SearchSymbolTool {
    fn name(&self) -> &str {
        "search_symbol"
    }

    fn label(&self) -> &str {
        "Search Symbol"
    }

    fn description(&self) -> &str {
        "Search for a cryptocurrency or stock by name or ticker. \
         Returns matching symbols with their IDs, which can be used with get_price. \
         Examples: 'bitcoin', 'BTC', 'apple', 'solana'"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query — a name, ticker, or partial match. e.g. 'bitcoin', 'sol', 'apple'"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query' parameter".into()))?
            .trim();

        if query.is_empty() {
            return Err(ToolError::InvalidArgs("query cannot be empty".into()));
        }

        let mut results = Vec::new();

        // Search CoinGecko
        match search_coingecko(&self.client, query).await {
            Ok(coins) => results.push(coins),
            Err(e) => results.push(format!("CoinGecko search error: {}", e)),
        }

        // Add common stock suggestions if query matches
        let stock_matches = search_common_stocks(query);
        if !stock_matches.is_empty() {
            results.push(stock_matches);
        }

        let output = results.join("\n\n");
        Ok(ToolResult {
            content: vec![Content::Text { text: output }],
            details: serde_json::json!({}),
        })
    }
}

async fn search_coingecko(client: &Client, query: &str) -> Result<String, String> {
    let url = format!("{}/search?query={}", COINGECKO_BASE, query);

    let data = fetch_json_with_retry(client, &url).await?;

    let coins = data["coins"]
        .as_array()
        .map(|arr| arr.iter().take(8).collect::<Vec<_>>())
        .unwrap_or_default();

    if coins.is_empty() {
        return Ok(format!("🔍 No crypto results for '{}'", query));
    }

    let mut output = format!("🪙 Crypto results for '{}':\n", query);
    for coin in &coins {
        let id = coin["id"].as_str().unwrap_or("?");
        let name = coin["name"].as_str().unwrap_or("?");
        let symbol = coin["symbol"].as_str().unwrap_or("?");
        let rank = coin["market_cap_rank"].as_u64();
        let rank_str = rank
            .map(|r| format!(" (rank #{})", r))
            .unwrap_or_default();

        output.push_str(&format!(
            "  {} ({}) — ID: \"{}\"{}\n",
            name,
            symbol.to_uppercase(),
            id,
            rank_str
        ));
    }
    output.push_str("\n💡 Use the 'id' value with get_price, e.g.: get_price(symbol=\"bitcoin\")");

    Ok(output)
}

/// Search a built-in list of common US stocks/indices
fn search_common_stocks(query: &str) -> String {
    let query_lower = query.to_lowercase();

    let stocks: Vec<(&str, &str, &str)> = vec![
        ("AAPL", "Apple Inc.", "Technology"),
        ("MSFT", "Microsoft Corporation", "Technology"),
        ("GOOGL", "Alphabet Inc. (Google)", "Technology"),
        ("AMZN", "Amazon.com Inc.", "Technology"),
        ("NVDA", "NVIDIA Corporation", "Technology"),
        ("META", "Meta Platforms (Facebook)", "Technology"),
        ("TSLA", "Tesla Inc.", "Automotive/Energy"),
        ("JPM", "JPMorgan Chase & Co.", "Finance"),
        ("V", "Visa Inc.", "Finance"),
        ("JNJ", "Johnson & Johnson", "Healthcare"),
        ("WMT", "Walmart Inc.", "Retail"),
        ("PG", "Procter & Gamble", "Consumer"),
        ("MA", "Mastercard Inc.", "Finance"),
        ("UNH", "UnitedHealth Group", "Healthcare"),
        ("HD", "The Home Depot", "Retail"),
        ("DIS", "The Walt Disney Company", "Entertainment"),
        ("NFLX", "Netflix Inc.", "Entertainment"),
        ("AMD", "Advanced Micro Devices", "Technology"),
        ("INTC", "Intel Corporation", "Technology"),
        ("CRM", "Salesforce Inc.", "Technology"),
        ("^GSPC", "S&P 500", "Index"),
        ("^IXIC", "NASDAQ Composite", "Index"),
        ("^DJI", "Dow Jones Industrial Average", "Index"),
        ("BTC-USD", "Bitcoin (on Yahoo)", "Crypto"),
        ("ETH-USD", "Ethereum (on Yahoo)", "Crypto"),
    ];

    let matches: Vec<&(&str, &str, &str)> = stocks
        .iter()
        .filter(|(ticker, name, sector)| {
            ticker.to_lowercase().contains(&query_lower)
                || name.to_lowercase().contains(&query_lower)
                || sector.to_lowercase().contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        return String::new();
    }

    let mut output = format!("📊 Stock/Index results for '{}':\n", query);
    for (ticker, name, sector) in &matches {
        output.push_str(&format!("  {} — {} [{}]\n", ticker, name, sector));
    }
    output.push_str("\n💡 Use ticker with get_price, e.g.: get_price(symbol=\"AAPL\")");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_common_stocks_by_ticker() {
        let result = search_common_stocks("AAPL");
        assert!(result.contains("Apple"));
    }

    #[test]
    fn test_search_common_stocks_by_name() {
        let result = search_common_stocks("apple");
        assert!(result.contains("AAPL"));
    }

    #[test]
    fn test_search_common_stocks_by_sector() {
        let result = search_common_stocks("technology");
        assert!(result.contains("AAPL"));
        assert!(result.contains("MSFT"));
    }

    #[test]
    fn test_search_common_stocks_no_match() {
        let result = search_common_stocks("zzzznonexistent");
        assert!(result.is_empty());
    }

    #[test]
    fn test_search_indices() {
        let result = search_common_stocks("S&P");
        assert!(result.contains("^GSPC"));
    }
}
