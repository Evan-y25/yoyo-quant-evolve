//! get_price_history tool — fetch OHLCV historical price data.
//!
//! Crypto: CoinGecko API (market_chart endpoint)
//! Stocks: Yahoo Finance (chart endpoint with various ranges)
//!
//! Returns price history with open/high/low/close/volume data,
//! formatted as an ASCII table and sparkline chart.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use yoagent::types::*;

use super::format::{format_change, format_large_number_usd, format_price, is_likely_stock_ticker};
use super::http::{create_client, fetch_json_with_retry};
use super::indicators;

const COINGECKO_BASE: &str = "https://api.coingecko.com/api/v3";
const YAHOO_CHART_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

pub struct GetPriceHistoryTool {
    client: Client,
}

impl GetPriceHistoryTool {
    pub fn new() -> Self {
        Self {
            client: create_client(),
        }
    }
}

#[async_trait]
impl AgentTool for GetPriceHistoryTool {
    fn name(&self) -> &str {
        "get_price_history"
    }

    fn label(&self) -> &str {
        "Price History"
    }

    fn description(&self) -> &str {
        "Fetch historical OHLCV (Open/High/Low/Close/Volume) price data for a cryptocurrency or stock. \
         Returns a summary table and ASCII sparkline chart. \
         Supports ranges: 1d, 7d, 30d, 90d, 1y. \
         For crypto, use CoinGecko IDs (e.g. 'bitcoin'). For stocks, use tickers (e.g. 'AAPL')."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "The asset to look up. For crypto, use CoinGecko ID (e.g. 'bitcoin'). For stocks, use ticker (e.g. 'AAPL')."
                },
                "range": {
                    "type": "string",
                    "enum": ["1d", "7d", "30d", "90d", "1y"],
                    "description": "Time range for history. Default: '30d'"
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

        let range = params["range"]
            .as_str()
            .unwrap_or("30d");

        let source = params["source"]
            .as_str()
            .unwrap_or("auto");

        let result = match source {
            "coingecko" => fetch_coingecko_history(&self.client, symbol, range).await,
            "yahoo" => fetch_yahoo_history(&self.client, symbol, range).await,
            _ => {
                if is_likely_stock_ticker(symbol) {
                    fetch_yahoo_history(&self.client, symbol, range).await
                } else {
                    match fetch_coingecko_history(&self.client, symbol, range).await {
                        Ok(text) => Ok(text),
                        Err(_) => {
                            let yahoo_symbol = format!("{}-USD", symbol.to_uppercase());
                            fetch_yahoo_history(&self.client, &yahoo_symbol, range).await
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
            Err(e) => Err(ToolError::Failed(format!("Failed to fetch price history: {}", e))),
        }
    }
}

/// Map our range strings to CoinGecko days parameter.
fn range_to_coingecko_days(range: &str) -> &str {
    match range {
        "1d" => "1",
        "7d" => "7",
        "30d" => "30",
        "90d" => "90",
        "1y" => "365",
        _ => "30",
    }
}

/// Map our range strings to Yahoo Finance range + interval.
fn range_to_yahoo_params(range: &str) -> (&str, &str) {
    match range {
        "1d" => ("1d", "5m"),
        "7d" => ("5d", "1h"),
        "30d" => ("1mo", "1d"),
        "90d" => ("3mo", "1d"),
        "1y" => ("1y", "1wk"),
        _ => ("1mo", "1d"),
    }
}

async fn fetch_coingecko_history(
    client: &Client,
    coin_id: &str,
    range: &str,
) -> Result<String, String> {
    let days = range_to_coingecko_days(range);
    let url = format!(
        "{}/coins/{}/market_chart?vs_currency=usd&days={}",
        COINGECKO_BASE,
        coin_id.to_lowercase(),
        days,
    );

    let data = fetch_json_with_retry(client, &url).await?;

    let prices = data["prices"]
        .as_array()
        .ok_or_else(|| format!("No price data found for '{}'. Use search_symbol to find the correct ID.", coin_id))?;

    let volumes = data["total_volumes"]
        .as_array();

    if prices.is_empty() {
        return Err("No historical data available".into());
    }

    // Extract price points: each is [timestamp_ms, price]
    let price_points: Vec<(i64, f64)> = prices
        .iter()
        .filter_map(|p| {
            let arr = p.as_array()?;
            let ts = arr.first()?.as_f64()? as i64;
            let price = arr.get(1)?.as_f64()?;
            Some((ts, price))
        })
        .collect();

    let volume_points: Vec<f64> = volumes
        .map(|v| {
            v.iter()
                .filter_map(|p| p.as_array()?.get(1)?.as_f64())
                .collect()
        })
        .unwrap_or_default();

    if price_points.is_empty() {
        return Err("No valid price data in response".into());
    }

    // Calculate OHLCV summary
    let first_price = price_points.first().unwrap().1;
    let last_price = price_points.last().unwrap().1;
    let high = price_points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let low = price_points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let total_vol: f64 = volume_points.iter().sum();
    let change_pct = ((last_price - first_price) / first_price) * 100.0;

    // Build output
    let mut output = format!(
        "📊 {} — {} Price History (CoinGecko)\n",
        coin_id, range
    );
    output.push_str("─────────────────────────────────────────\n");
    output.push_str(&format!("  Open:   {}\n", format_price(first_price)));
    output.push_str(&format!("  High:   {}\n", format_price(high)));
    output.push_str(&format!("  Low:    {}\n", format_price(low)));
    output.push_str(&format!("  Close:  {}\n", format_price(last_price)));
    output.push_str(&format!("  Change: {}\n", format_change(change_pct)));
    if total_vol > 0.0 {
        output.push_str(&format!("  Volume: {}\n", format_large_number_usd(total_vol)));
    }
    output.push_str("─────────────────────────────────────────\n");

    // ASCII sparkline
    let prices_only: Vec<f64> = price_points.iter().map(|p| p.1).collect();
    output.push_str(&sparkline(&prices_only, 50));

    // Technical indicators (only if we have enough data)
    let vol_ref = if volume_points.is_empty() { None } else { Some(volume_points.as_slice()) };
    output.push_str(&format_indicators(&prices_only, last_price, vol_ref));

    Ok(output)
}

async fn fetch_yahoo_history(
    client: &Client,
    symbol: &str,
    range: &str,
) -> Result<String, String> {
    let (yahoo_range, interval) = range_to_yahoo_params(range);
    let url = format!(
        "{}/{}?range={}&interval={}",
        YAHOO_CHART_BASE, symbol, yahoo_range, interval
    );

    let data = fetch_json_with_retry(client, &url).await?;

    let result = data["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("No data found for '{}'", symbol))?;

    let meta = &result["meta"];
    let name = meta["shortName"]
        .as_str()
        .or_else(|| meta["symbol"].as_str())
        .unwrap_or(symbol);
    let currency = meta["currency"].as_str().unwrap_or("USD");

    let indicators = &result["indicators"]["quote"]
        .as_array()
        .and_then(|arr| arr.first());

    let timestamps = result["timestamp"]
        .as_array()
        .ok_or("No timestamp data")?;

    let closes: Vec<f64> = indicators
        .and_then(|q| q["close"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let opens: Vec<f64> = indicators
        .and_then(|q| q["open"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let highs: Vec<f64> = indicators
        .and_then(|q| q["high"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let lows: Vec<f64> = indicators
        .and_then(|q| q["low"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let volumes: Vec<f64> = indicators
        .and_then(|q| q["volume"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    if closes.is_empty() {
        return Err("No close price data available".into());
    }

    let first_close = opens.first().copied().unwrap_or(closes[0]);
    let last_close = *closes.last().unwrap();
    let period_high = highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let period_low = lows.iter().copied().fold(f64::INFINITY, f64::min);
    let total_vol: f64 = volumes.iter().sum();
    let change_pct = ((last_close - first_close) / first_close) * 100.0;

    let mut output = format!(
        "📊 {} — {} ({}) — {} Price History (Yahoo Finance)\n",
        symbol.to_uppercase(),
        name,
        currency,
        range,
    );
    output.push_str("─────────────────────────────────────────\n");
    output.push_str(&format!("  Open:     {}\n", format_price(first_close)));
    output.push_str(&format!("  High:     {}\n", format_price(period_high)));
    output.push_str(&format!("  Low:      {}\n", format_price(period_low)));
    output.push_str(&format!("  Close:    {}\n", format_price(last_close)));
    output.push_str(&format!("  Change:   {}\n", format_change(change_pct)));
    if total_vol > 0.0 {
        output.push_str(&format!("  Volume:   {}\n", format_large_number_usd(total_vol)));
    }
    output.push_str(&format!("  Periods:  {}\n", timestamps.len()));
    output.push_str("─────────────────────────────────────────\n");

    // ASCII sparkline from close prices
    output.push_str(&sparkline(&closes, 50));

    // Technical indicators
    // Technical indicators
    let vol_ref = if volumes.is_empty() { None } else { Some(volumes.as_slice()) };
    output.push_str(&format_indicators(&closes, last_close, vol_ref));

    Ok(output)
}

/// Format technical indicators for display.
/// Only shows indicators when there's enough data.
fn format_indicators(prices: &[f64], current_price: f64, volumes: Option<&[f64]>) -> String {
    let mut output = String::new();
    let has_any = prices.len() >= 15; // Need at least 15 for RSI(14)

    if !has_any {
        return output;
    }

    output.push_str("─────────────────────────────────────────\n");
    output.push_str("  📐 Technical Indicators\n");

    // SMA
    if let Some(sma7) = indicators::sma(prices, 7) {
        output.push_str(&format!("  SMA(7):   {}\n", format_price(sma7)));
    }
    if let Some(sma20) = indicators::sma(prices, 20) {
        output.push_str(&format!("  SMA(20):  {}\n", format_price(sma20)));

        // Show trend signal if we have both SMAs
        if let Some(sma7) = indicators::sma(prices, 7) {
            output.push_str(&format!("  Trend:    {}\n", indicators::sma_signal(current_price, sma7, sma20)));
        }
    }

    // EMA
    if let Some(ema12) = indicators::ema(prices, 12) {
        output.push_str(&format!("  EMA(12):  {}\n", format_price(ema12)));
    }

    // RSI
    if let Some(rsi_val) = indicators::rsi(prices, 14) {
        output.push_str(&format!(
            "  RSI(14):  {:.1} {}\n",
            rsi_val,
            indicators::rsi_signal(rsi_val)
        ));
    }

    // MACD (standard 12/26/9)
    if let Some(macd_result) = indicators::macd(prices, 12, 26, 9) {
        output.push_str(&format!(
            "  MACD:     {:.4} | Signal: {:.4} | Hist: {:.4}\n",
            macd_result.macd_line,
            macd_result.signal_line,
            macd_result.histogram,
        ));
        output.push_str(&format!(
            "  MACD:     {}\n",
            indicators::macd_signal(&macd_result)
        ));
    }

    // Bollinger Bands (standard 20, 2σ)
    if let Some(bb) = indicators::bollinger_bands(prices, 20, 2.0) {
        output.push_str(&format!(
            "  BB(20):   Upper: {} | Mid: {} | Lower: {}\n",
            format_price(bb.upper),
            format_price(bb.middle),
            format_price(bb.lower),
        ));
        output.push_str(&format!(
            "  BB:       %B: {:.2} | BW: {:.2}% {}\n",
            bb.percent_b,
            bb.bandwidth,
            indicators::bollinger_signal(&bb),
        ));
    }

    // VWAP (Volume Weighted Average Price) — only when volume data is available
    if let Some(vols) = volumes {
        if let Some(vwap_val) = indicators::vwap(prices, vols) {
            output.push_str(&format!(
                "  VWAP:     {} {}\n",
                format_price(vwap_val),
                indicators::vwap_signal(current_price, vwap_val),
            ));
        }
    }

    output
}

/// Generate an ASCII sparkline chart from price data.
/// Width is the number of characters wide the chart should be.
fn sparkline(prices: &[f64], width: usize) -> String {
    if prices.is_empty() {
        return String::from("  (no data for chart)\n");
    }

    // Downsample if needed
    let sampled = if prices.len() > width {
        let step = prices.len() as f64 / width as f64;
        (0..width)
            .map(|i| {
                let idx = (i as f64 * step) as usize;
                prices[idx.min(prices.len() - 1)]
            })
            .collect::<Vec<f64>>()
    } else {
        prices.to_vec()
    };

    let min = sampled.iter().copied().fold(f64::INFINITY, f64::min);
    let max = sampled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    if range == 0.0 {
        return format!("  {}\n  (flat)\n", "▅".repeat(sampled.len()));
    }

    // Use Unicode block characters for 8-level resolution
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut chart = String::from("  ");
    for &price in &sampled {
        let normalized = ((price - min) / range * 7.0) as usize;
        let idx = normalized.min(7);
        chart.push(blocks[idx]);
    }
    chart.push('\n');

    // Add min/max labels
    chart.push_str(&format!(
        "  {:<25} {:>25}\n",
        format!("Low: {}", format_price(min)),
        format!("High: {}", format_price(max)),
    ));

    chart
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_to_coingecko_days() {
        assert_eq!(range_to_coingecko_days("1d"), "1");
        assert_eq!(range_to_coingecko_days("7d"), "7");
        assert_eq!(range_to_coingecko_days("30d"), "30");
        assert_eq!(range_to_coingecko_days("90d"), "90");
        assert_eq!(range_to_coingecko_days("1y"), "365");
        assert_eq!(range_to_coingecko_days("unknown"), "30");
    }

    #[test]
    fn test_range_to_yahoo_params() {
        let (range, interval) = range_to_yahoo_params("1d");
        assert_eq!(range, "1d");
        assert_eq!(interval, "5m");

        let (range, interval) = range_to_yahoo_params("30d");
        assert_eq!(range, "1mo");
        assert_eq!(interval, "1d");
    }

    #[test]
    fn test_sparkline_basic() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sparkline(&prices, 50);
        assert!(result.contains('▁'));
        assert!(result.contains('█'));
        assert!(result.contains("Low:"));
        assert!(result.contains("High:"));
    }

    #[test]
    fn test_sparkline_flat() {
        let prices = vec![100.0, 100.0, 100.0];
        let result = sparkline(&prices, 50);
        assert!(result.contains("flat"));
    }

    #[test]
    fn test_sparkline_empty() {
        let result = sparkline(&[], 50);
        assert!(result.contains("no data"));
    }

    #[test]
    fn test_sparkline_single_point() {
        let prices = vec![42.0];
        let result = sparkline(&prices, 50);
        // Single point means range = 0, should show flat
        assert!(result.contains("flat"));
    }

    #[test]
    fn test_sparkline_downsampling() {
        // 100 data points, width of 10
        let prices: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = sparkline(&prices, 10);
        assert!(result.contains('▁'));
        assert!(result.contains('█'));
    }

    #[test]
    fn test_is_likely_stock_ticker() {
        assert!(is_likely_stock_ticker("AAPL"));
        assert!(is_likely_stock_ticker("^GSPC"));
        assert!(is_likely_stock_ticker("BTC-USD"));
        assert!(!is_likely_stock_ticker("bitcoin"));
        assert!(!is_likely_stock_ticker("ethereum"));
    }
}
