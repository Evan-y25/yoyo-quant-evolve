//! HTTP client utilities with retry logic and rate limiting.
//!
//! CoinGecko's free API has aggressive rate limits (10-30 req/min).
//! This module provides a shared HTTP client with automatic retry
//! on transient failures (429, 5xx, timeouts).

use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

const USER_AGENT: &str = "yoyo-trading-agent/0.1";
const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 500;

/// Create a shared HTTP client with sensible defaults.
pub fn create_client() -> Client {
    create_client_with_timeout(DEFAULT_TIMEOUT_SECS)
}

/// Create a shared HTTP client with custom timeout.
pub fn create_client_with_timeout(timeout_secs: u64) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to create HTTP client")
}

/// Fetch JSON from a URL with automatic retry on transient errors.
/// Retries on: 429 (rate limit), 500-599 (server errors), timeouts.
pub async fn fetch_json_with_retry(client: &Client, url: &str) -> Result<Value, String> {
    let mut last_error = String::new();

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = BASE_RETRY_DELAY_MS * 2u64.pow(attempt - 1);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();

                if status.is_success() {
                    return resp
                        .json::<Value>()
                        .await
                        .map_err(|e| format!("Failed to parse response: {}", e));
                }

                if status.as_u16() == 429 {
                    last_error =
                        format!("Rate limited (429). Retry {}/{}", attempt + 1, MAX_RETRIES);
                    continue;
                }

                if status.is_server_error() {
                    last_error = format!(
                        "Server error ({}). Retry {}/{}",
                        status,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    continue;
                }

                // Client errors (4xx except 429) are not retried
                return Err(format!("API returned status {}", status));
            }
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    last_error = format!(
                        "Network error: {}. Retry {}/{}",
                        e,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    continue;
                }
                return Err(format!("HTTP request failed: {}", e));
            }
        }
    }

    Err(format!(
        "Failed after {} retries. Last error: {}",
        MAX_RETRIES, last_error
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let _client = create_client();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_create_client_with_timeout() {
        let _client = create_client_with_timeout(30);
        // Just verify it doesn't panic
    }
}
