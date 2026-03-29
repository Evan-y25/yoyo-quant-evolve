//! get_price tool — fetch real-time cryptocurrency and stock prices.
//!
//! Crypto: CoinGecko API (free, no key needed)
//! Stocks: Yahoo Finance (unofficial, no key needed)

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use yoagent::types::*;

use super::format::{change_emoji, format_change, format_large_number_usd, format_price};

const COINGECKO_BASE: &str = "https://api.coingecko.com/api/v3";
const YAHOO_CHART_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const USER_AGENT: &str = "yoyo-trading-agent/0.1";
const TIMEOUT_SECS: u64 = 10;

pub struct GetPriceTool {
    client: Client,
}

impl GetPriceTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .user_agent(USER_AGENT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[async_trait]
impl AgentTool for GetPriceTool {
    fn name(&self) -> &str {
        "get_price"
    }

    fn label(&self) -> &str {
        "Get Price"
    }

    fn description(&self) -> &str {
        "Fetch the current price of a cryptocurrency or stock. \
         For crypto, use CoinGecko IDs like 'bitcoin', 'ethereum', 'solana'. \
         For stocks, use tickers like 'AAPL', 'MSFT', 'GOOGL'. \
         Returns price, 24h change, market cap, and volume."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "The asset to look up. For crypto, use CoinGecko ID (e.g. 'bitcoin', 'ethereum'). For stocks, use ticker (e.g. 'AAPL', 'TSLA'). For crypto on Yahoo, use 'BTC-USD' format."
                },
                "source": {
                    "type": "string",
                    "enum": ["auto", "coingecko", "yahoo"],
                    "description": "Data source. 'auto' will try CoinGecko for known crypto, Yahoo for stocks. Default: 'auto'"
                }
            },
            "required": ["symbol"]
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let symbol = params["symbol"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'symbol' parameter".into()))?
            .trim();

        let source = params["source"]
            .as_str()
            .unwrap_or("auto");

        let result = match source {
            "coingecko" => fetch_coingecko_price(&self.client, symbol).await,
            "yahoo" => fetch_yahoo_price(&self.client, symbol).await,
            _ => {
                // Auto-detect: if it looks like a stock ticker (all uppercase, 1-5 chars), try Yahoo
                // Otherwise try CoinGecko first
                if is_likely_stock_ticker(symbol) {
                    fetch_yahoo_price(&self.client, symbol).await
                } else {
                    match fetch_coingecko_price(&self.client, symbol).await {
                        Ok(text) => Ok(text),
                        Err(_) => {
                            // Fallback: try Yahoo with -USD suffix for crypto tickers
                            let yahoo_symbol = format!("{}-USD", symbol.to_uppercase());
                            fetch_yahoo_price(&self.client, &yahoo_symbol).await
                        }
                    }
                }
            }
        };

        match result {
            Ok(text) => Ok(ToolResult {
                content: vec![Content::Text { text }],
                details: serde_json::json!({}),
            }),
            Err(e) => Err(ToolError::Failed(format!("Failed to fetch price: {}", e))),
        }
    }
}

/// Heuristic: stock tickers are 1-5 uppercase letters, or contain special chars like ^ or .
fn is_likely_stock_ticker(s: &str) -> bool {
    let s = s.trim();
    if s.starts_with('^') || s.contains('.') || s.contains('-') {
        return true;
    }
    s.len() <= 5 && s.chars().all(|c| c.is_ascii_uppercase())
}

async fn fetch_coingecko_price(client: &Client, coin_id: &str) -> Result<String, String> {
    let url = format!(
        "{}/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true&include_market_cap=true&include_24hr_vol=true",
        COINGECKO_BASE,
        coin_id.to_lowercase()
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CoinGecko API returned status {}", resp.status()));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let coin_data = data.get(coin_id.to_lowercase().as_str())
        .ok_or_else(|| format!("No data found for '{}'. Try using the CoinGecko ID (e.g., 'bitcoin' not 'BTC'). Use the search_symbol tool to find the correct ID.", coin_id))?;

    let price = coin_data["usd"].as_f64().unwrap_or(0.0);
    let change_24h = coin_data["usd_24h_change"].as_f64().unwrap_or(0.0);
    let market_cap = coin_data["usd_market_cap"].as_f64().unwrap_or(0.0);
    let volume_24h = coin_data["usd_24h_vol"].as_f64().unwrap_or(0.0);

    Ok(format!(
        "{} {} (CoinGecko)\n\
         Price: {}\n\
         24h Change: {}\n\
         Market Cap: {}\n\
         24h Volume: {}",
        change_emoji(change_24h),
        coin_id,
        format_price(price),
        format_change(change_24h),
        format_large_number_usd(market_cap),
        format_large_number_usd(volume_24h),
    ))
}

async fn fetch_yahoo_price(client: &Client, symbol: &str) -> Result<String, String> {
    let url = format!(
        "{}/{}?range=1d&interval=5m&symbols={}",
        YAHOO_CHART_BASE, symbol, symbol
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Yahoo Finance API returned status {}", resp.status()));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let result = data["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("No data found for '{}'", symbol))?;

    let meta = &result["meta"];
    let price = meta["regularMarketPrice"].as_f64().unwrap_or(0.0);
    let prev_close = meta["chartPreviousClose"].as_f64().unwrap_or(0.0);
    let currency = meta["currency"].as_str().unwrap_or("USD");
    let exchange = meta["exchangeName"].as_str().unwrap_or("Unknown");
    let name = meta["shortName"]
        .as_str()
        .or_else(|| meta["symbol"].as_str())
        .unwrap_or(symbol);

    let change = if prev_close > 0.0 {
        ((price - prev_close) / prev_close) * 100.0
    } else {
        0.0
    };
    let change_abs = price - prev_close;

    Ok(format!(
        "{} {} — {} (Yahoo Finance)\n\
         Price: {} {:.2}\n\
         Change: {}{:.2} ({})\n\
         Exchange: {}",
        change_emoji(change),
        symbol.to_uppercase(),
        name,
        currency,
        price,
        if change_abs >= 0.0 { "+" } else { "" },
        change_abs,
        format_change(change),
        exchange,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_stock_ticker() {
        assert!(is_likely_stock_ticker("AAPL"));
        assert!(is_likely_stock_ticker("MSFT"));
        assert!(is_likely_stock_ticker("TSLA"));
        assert!(is_likely_stock_ticker("^GSPC"));
        assert!(is_likely_stock_ticker("BRK.B"));
        assert!(is_likely_stock_ticker("BTC-USD"));
        assert!(!is_likely_stock_ticker("bitcoin"));
        assert!(!is_likely_stock_ticker("ethereum"));
        assert!(!is_likely_stock_ticker("solana"));
    }

    #[test]
    fn test_coingecko_url_construction() {
        let coin_id = "bitcoin";
        let url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true&include_market_cap=true&include_24hr_vol=true",
            COINGECKO_BASE,
            coin_id.to_lowercase()
        );
        assert!(url.contains("api.coingecko.com"));
        assert!(url.contains("bitcoin"));
    }
}
