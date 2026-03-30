//! get_news tool — fetch market-relevant news headlines.
//!
//! Uses Google News RSS feed (no API key needed) for market news.
//! Also integrates CoinGecko trending data for crypto-specific sentiment.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use yoagent::types::*;

use super::http::create_client;

const GOOGLE_NEWS_RSS: &str = "https://news.google.com/rss/search";

pub struct GetNewsTool {
    client: Client,
}

impl GetNewsTool {
    pub fn new() -> Self {
        Self {
            client: create_client(),
        }
    }
}

#[async_trait]
impl AgentTool for GetNewsTool {
    fn name(&self) -> &str {
        "get_news"
    }

    fn label(&self) -> &str {
        "Get News"
    }

    fn description(&self) -> &str {
        "Fetch recent market news headlines for a given topic (e.g. 'bitcoin', 'AAPL', 'crypto market'). \
         Returns the latest headlines with source and publication time. \
         Useful for understanding what's driving price movements."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "News search query. E.g. 'bitcoin', 'ethereum price', 'AAPL earnings', 'crypto market', 'stock market today'"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of headlines to return (default: 10, max: 20)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, _ctx: ToolContext) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query' parameter".into()))?
            .trim();

        if query.is_empty() {
            return Err(ToolError::InvalidArgs("query cannot be empty".into()));
        }

        let limit = params["limit"].as_u64().unwrap_or(10).min(20) as usize;

        let result = fetch_google_news(&self.client, query, limit).await;

        match result {
            Ok(text) => Ok(ToolResult {
                content: vec![Content::Text { text }],
                details: serde_json::json!({}),
            }),
            Err(e) => Err(ToolError::Failed(format!("Failed to fetch news: {}", e))),
        }
    }
}

async fn fetch_google_news(client: &Client, query: &str, limit: usize) -> Result<String, String> {
    let url = format!(
        "{}?q={}&hl=en-US&gl=US&ceid=US:en",
        GOOGLE_NEWS_RSS,
        urlencoding::encode(query),
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch news: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Google News returned status {}", resp.status()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse RSS XML manually (we don't want to add a full XML parser dependency)
    let items = parse_rss_items(&body, limit);

    if items.is_empty() {
        return Ok(format!("📰 No recent news found for '{}'", query));
    }

    let mut output = format!(
        "📰 Latest news for '{}' ({} headlines)\n",
        query,
        items.len()
    );
    output.push_str("─────────────────────────────────────────────────────────────\n");

    for (i, item) in items.iter().enumerate() {
        let num = i + 1;
        output.push_str(&format!(
            "  {}. {}\n     📌 {} — {}\n\n",
            num, item.title, item.source, item.pub_date,
        ));
    }

    output.push_str("─────────────────────────────────────────────────────────────\n");
    output.push_str("💡 News from Google News RSS. Headlines may affect market sentiment.");

    Ok(output)
}

struct NewsItem {
    title: String,
    source: String,
    pub_date: String,
}

/// Simple RSS XML parser — extracts title, source, and pubDate from <item> elements.
/// We do this without a full XML crate to keep dependencies minimal.
fn parse_rss_items(xml: &str, limit: usize) -> Vec<NewsItem> {
    let mut items = Vec::new();

    // Split by <item> tags
    let parts: Vec<&str> = xml.split("<item>").collect();

    // Skip the first part (before any <item>)
    for part in parts.iter().skip(1).take(limit) {
        let title = extract_tag_content(part, "title")
            .unwrap_or_default()
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'");

        let source = extract_tag_content(part, "source").unwrap_or_else(|| "Unknown".to_string());

        let pub_date = extract_tag_content(part, "pubDate")
            .map(|d| format_pub_date(&d))
            .unwrap_or_else(|| "Unknown".to_string());

        if !title.is_empty() {
            items.push(NewsItem {
                title,
                source,
                pub_date,
            });
        }
    }

    items
}

/// Extract the text content between <tag> and </tag>.
fn extract_tag_content(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);

    let start_idx = xml.find(&open)?;
    // Find the > that closes the opening tag (handles attributes like <source url="...">)
    let content_start = xml[start_idx..].find('>')? + start_idx + 1;
    let end_idx = xml[content_start..].find(&close)? + content_start;

    Some(xml[content_start..end_idx].trim().to_string())
}

/// Format RFC 2822 date into a shorter, more readable form.
/// Input: "Mon, 30 Mar 2026 02:10:00 GMT"
/// Output: "Mar 30, 02:10 UTC"
fn format_pub_date(date_str: &str) -> String {
    // Try to parse the common RSS date format
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 5 {
        // "Mon, 30 Mar 2026 02:10:00 GMT"
        // parts: ["Mon,", "30", "Mar", "2026", "02:10:00", "GMT"]
        let day = parts[1];
        let month = parts[2];
        let time = parts[4];
        // Take only HH:MM from time
        let short_time = if time.len() >= 5 { &time[..5] } else { time };
        format!("{} {}, {} UTC", month, day, short_time)
    } else {
        date_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag_content_simple() {
        let xml = "<title>Hello World</title>";
        assert_eq!(
            extract_tag_content(xml, "title"),
            Some("Hello World".to_string())
        );
    }

    #[test]
    fn test_extract_tag_content_with_attributes() {
        let xml = r#"<source url="https://example.com">Example News</source>"#;
        assert_eq!(
            extract_tag_content(xml, "source"),
            Some("Example News".to_string())
        );
    }

    #[test]
    fn test_extract_tag_content_missing() {
        let xml = "<title>Hello</title>";
        assert_eq!(extract_tag_content(xml, "description"), None);
    }

    #[test]
    fn test_format_pub_date() {
        let date = "Mon, 30 Mar 2026 02:10:00 GMT";
        assert_eq!(format_pub_date(date), "Mar 30, 02:10 UTC");
    }

    #[test]
    fn test_format_pub_date_short() {
        let date = "unknown";
        assert_eq!(format_pub_date(date), "unknown");
    }

    #[test]
    fn test_parse_rss_items() {
        let xml = r#"
            <channel>
            <item>
                <title>Bitcoin hits $100K</title>
                <source url="https://example.com">CoinDesk</source>
                <pubDate>Mon, 30 Mar 2026 02:10:00 GMT</pubDate>
            </item>
            <item>
                <title>Ethereum upgrade &amp; news</title>
                <source url="https://example.com">CryptoSlate</source>
                <pubDate>Sun, 29 Mar 2026 15:00:00 GMT</pubDate>
            </item>
            </channel>
        "#;

        let items = parse_rss_items(xml, 10);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Bitcoin hits $100K");
        assert_eq!(items[0].source, "CoinDesk");
        assert_eq!(items[1].title, "Ethereum upgrade & news");
    }

    #[test]
    fn test_parse_rss_items_limit() {
        let xml = r#"
            <item><title>News 1</title><source>A</source><pubDate>Mon, 01 Jan 2026 00:00:00 GMT</pubDate></item>
            <item><title>News 2</title><source>B</source><pubDate>Mon, 01 Jan 2026 00:00:00 GMT</pubDate></item>
            <item><title>News 3</title><source>C</source><pubDate>Mon, 01 Jan 2026 00:00:00 GMT</pubDate></item>
        "#;

        let items = parse_rss_items(xml, 2);
        assert_eq!(items.len(), 2);
    }
}
