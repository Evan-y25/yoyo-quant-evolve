//! Shared formatting utilities for trading tools.
//!
//! These are used across multiple tools to ensure consistent display of
//! prices, large numbers, and percentage changes.

/// Format a price with adaptive decimal places based on magnitude.
/// - >= $1: 2 decimal places with comma separators ($87,432.15)
/// - >= $0.01: 4 decimal places ($0.5000)
/// - < $0.01: 6 decimal places ($0.000450)
pub fn format_price(price: f64) -> String {
    if price >= 1.0 {
        let integer_part = price as u64;
        let decimal_part = format!("{:.2}", price.fract());
        // decimal_part is "0.XX" — take the ".XX" part
        let decimal_str = &decimal_part[1..]; // ".XX"
        format!("${}{}", format_with_commas(integer_part), decimal_str)
    } else if price >= 0.01 {
        format!("${:.4}", price)
    } else {
        format!("${:.6}", price)
    }
}

/// Format an integer with comma separators (e.g., 87432 → "87,432").
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes: Vec<u8> = s.bytes().collect();
    let mut result = String::new();
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(b as char);
    }
    result
}

/// Format a large number with human-readable suffixes (T, B, M, K).
/// Prefixed with $ for monetary values.
pub fn format_large_number_usd(n: f64) -> String {
    if n >= 1_000_000_000_000.0 {
        format!("${:.2}T", n / 1_000_000_000_000.0)
    } else if n >= 1_000_000_000.0 {
        format!("${:.2}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("${:.2}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("${:.2}K", n / 1_000.0)
    } else {
        format!("${:.2}", n)
    }
}

/// Format a percentage change with sign and emoji.
pub fn format_change(change_pct: f64) -> String {
    format!(
        "{}{:.2}%",
        if change_pct >= 0.0 { "+" } else { "" },
        change_pct
    )
}

/// Get the appropriate emoji for a price change direction.
pub fn change_emoji(change_pct: f64) -> &'static str {
    if change_pct >= 0.0 {
        "📈"
    } else {
        "📉"
    }
}

/// Get the colored circle emoji for a change direction.
pub fn change_dot(change_pct: f64) -> &'static str {
    if change_pct >= 0.0 {
        "🟢"
    } else {
        "🔴"
    }
}

/// Heuristic: stock tickers are 1-5 uppercase letters, or contain special chars like ^ or .
/// Used by multiple tools to decide whether to query Yahoo Finance vs CoinGecko.
pub fn is_likely_stock_ticker(s: &str) -> bool {
    let s = s.trim();
    if s.starts_with('^') || s.contains('.') || s.contains('-') {
        return true;
    }
    s.len() <= 5 && s.chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_price_large() {
        assert_eq!(format_price(87432.15), "$87,432.15");
    }

    #[test]
    fn test_format_price_medium() {
        assert_eq!(format_price(0.5), "$0.5000");
    }

    #[test]
    fn test_format_price_tiny() {
        assert_eq!(format_price(0.00045), "$0.000450");
    }

    #[test]
    fn test_format_price_one_dollar() {
        assert_eq!(format_price(1.0), "$1.00");
    }

    #[test]
    fn test_format_price_thousands() {
        assert_eq!(format_price(1234.56), "$1,234.56");
    }

    #[test]
    fn test_format_price_millions() {
        assert_eq!(format_price(1234567.89), "$1,234,567.89");
    }

    #[test]
    fn test_format_with_commas() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(999), "999");
        assert_eq!(format_with_commas(1000), "1,000");
        assert_eq!(format_with_commas(1234567), "1,234,567");
        assert_eq!(format_with_commas(1000000000), "1,000,000,000");
    }

    #[test]
    fn test_format_large_number_trillion() {
        assert_eq!(format_large_number_usd(1_500_000_000_000.0), "$1.50T");
    }

    #[test]
    fn test_format_large_number_billion() {
        assert_eq!(format_large_number_usd(45_000_000_000.0), "$45.00B");
    }

    #[test]
    fn test_format_large_number_million() {
        assert_eq!(format_large_number_usd(1_200_000.0), "$1.20M");
    }

    #[test]
    fn test_format_large_number_thousand() {
        assert_eq!(format_large_number_usd(50_000.0), "$50.00K");
    }

    #[test]
    fn test_format_large_number_small() {
        assert_eq!(format_large_number_usd(42.5), "$42.50");
    }

    #[test]
    fn test_format_change_positive() {
        assert_eq!(format_change(5.5), "+5.50%");
    }

    #[test]
    fn test_format_change_negative() {
        assert_eq!(format_change(-3.2), "-3.20%");
    }

    #[test]
    fn test_format_change_zero() {
        assert_eq!(format_change(0.0), "+0.00%");
    }

    #[test]
    fn test_change_emoji() {
        assert_eq!(change_emoji(5.0), "📈");
        assert_eq!(change_emoji(-5.0), "📉");
        assert_eq!(change_emoji(0.0), "📈");
    }

    #[test]
    fn test_change_dot() {
        assert_eq!(change_dot(5.0), "🟢");
        assert_eq!(change_dot(-5.0), "🔴");
    }

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
}
