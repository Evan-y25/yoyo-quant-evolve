//! get_market_overview tool — top crypto by market cap + major US indices.
//!
//! Gives a snapshot of what's happening in both crypto and stock markets.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use yoagent::types::*;

use super::format::{change_dot, format_change, format_large_number_usd, format_price};
use super::http::{create_client_with_timeout, fetch_json_with_retry};

const COINGECKO_BASE: &str = "https://api.coingecko.com/api/v3";
const YAHOO_CHART_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

pub struct GetMarketOverviewTool {
    client: Client,
}

impl GetMarketOverviewTool {
    pub fn new() -> Self {
        Self {
            client: create_client_with_timeout(15),
        }
    }
}

#[async_trait]
impl AgentTool for GetMarketOverviewTool {
    fn name(&self) -> &str {
        "get_market_overview"
    }

    fn label(&self) -> &str {
        "Market Overview"
    }

    fn description(&self) -> &str {
        "Get a market overview showing top cryptocurrencies by market cap and major US stock indices. \
         Provides a quick snapshot of overall market conditions."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Number of top cryptocurrencies to show (default: 10, max: 25)"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let limit = params["limit"]
            .as_u64()
            .unwrap_or(10)
            .min(25) as usize;

        let mut sections = Vec::new();

        // Fetch top crypto
        match fetch_top_crypto(&self.client, limit).await {
            Ok(text) => sections.push(text),
            Err(e) => sections.push(format!("❌ Crypto data unavailable: {}", e)),
        }

        // Fetch major indices
        match fetch_major_indices(&self.client).await {
            Ok(text) => sections.push(text),
            Err(e) => sections.push(format!("❌ Index data unavailable: {}", e)),
        }

        let output = sections.join("\n\n");
        Ok(ToolResult {
            content: vec![Content::Text { text: output }],
            details: serde_json::json!({}),
        })
    }
}

async fn fetch_top_crypto(client: &Client, limit: usize) -> Result<String, String> {
    let url = format!(
        "{}/coins/markets?vs_currency=usd&order=market_cap_desc&per_page={}&page=1&sparkline=false&price_change_percentage=24h",
        COINGECKO_BASE, limit
    );

    let data = fetch_json_with_retry(client, &url).await?;

    let coins = data.as_array().ok_or("Expected array from CoinGecko markets endpoint")?;

    if coins.is_empty() {
        return Ok("🪙 No crypto data available".into());
    }

    let mut output = String::from("🪙 Top Cryptocurrencies by Market Cap\n");
    output.push_str("─────────────────────────────────────────────────────────────\n");
    output.push_str(&format!(
        "{:<4} {:<14} {:>12} {:>10} {:>14}\n",
        "#", "Name", "Price", "24h %", "Market Cap"
    ));
    output.push_str("─────────────────────────────────────────────────────────────\n");

    for coin in coins {
        let rank = coin["market_cap_rank"].as_u64().unwrap_or(0);
        let name = coin["name"].as_str().unwrap_or("?");
        let symbol = coin["symbol"].as_str().unwrap_or("?").to_uppercase();
        let price = coin["current_price"].as_f64().unwrap_or(0.0);
        let change = coin["price_change_percentage_24h"].as_f64().unwrap_or(0.0);
        let mcap = coin["market_cap"].as_f64().unwrap_or(0.0);

        let change_str = format_change(change);

        let display_name = if name.len() > 12 {
            format!("{}..", &name[..10])
        } else {
            name.to_string()
        };

        output.push_str(&format!(
            "{:<4} {:<10} {} {:>12} {:>10} {:>14}\n",
            rank,
            format!("{} ({})", display_name, symbol),
            change_dot(change),
            format_price(price),
            change_str,
            format_large_number_usd(mcap),
        ));
    }

    Ok(output)
}

async fn fetch_major_indices(client: &Client) -> Result<String, String> {
    let indices = vec![
        ("^GSPC", "S&P 500"),
        ("^IXIC", "NASDAQ"),
        ("^DJI", "Dow Jones"),
    ];

    let mut output = String::from("📊 Major US Indices\n");
    output.push_str("─────────────────────────────────────────\n");

    let mut any_success = false;

    for (symbol, name) in &indices {
        match fetch_yahoo_index(client, symbol).await {
            Ok((price, change_pct)) => {
                any_success = true;
                output.push_str(&format!(
                    "  {} {:<12} {:>12} {:>8}\n",
                    change_dot(change_pct),
                    name,
                    format_price(price),
                    format_change(change_pct),
                ));
            }
            Err(e) => {
                output.push_str(&format!("  ❌ {:<12} {}\n", name, e));
            }
        }
    }

    if !any_success {
        return Err("Could not fetch any index data".into());
    }

    Ok(output)
}

async fn fetch_yahoo_index(client: &Client, symbol: &str) -> Result<(f64, f64), String> {
    let url = format!("{}/{}?range=1d&interval=5m", YAHOO_CHART_BASE, symbol);

    let data = fetch_json_with_retry(client, &url).await?;

    let meta = &data["chart"]["result"][0]["meta"];
    let price = meta["regularMarketPrice"]
        .as_f64()
        .ok_or("no price data")?;
    let prev_close = meta["chartPreviousClose"]
        .as_f64()
        .unwrap_or(price);

    let change_pct = if prev_close > 0.0 {
        ((price - prev_close) / prev_close) * 100.0
    } else {
        0.0
    };

    Ok((price, change_pct))
}

#[cfg(test)]
mod tests {
    use crate::tools::format::*;

    #[test]
    fn test_format_price_large() {
        assert_eq!(format_price(87432.15), "$87,432.15");
    }

    #[test]
    fn test_format_price_small() {
        assert_eq!(format_price(0.5), "$0.5000");
    }

    #[test]
    fn test_format_price_tiny() {
        assert_eq!(format_price(0.00045), "$0.000450");
    }

    #[test]
    fn test_format_large_number() {
        assert_eq!(format_large_number_usd(1_700_000_000_000.0), "$1.70T");
        assert_eq!(format_large_number_usd(45_000_000_000.0), "$45.00B");
        assert_eq!(format_large_number_usd(500_000.0), "$500.00K");
    }
}
