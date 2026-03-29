//! Shared formatting utilities for trading tools.
//!
//! These are used across multiple tools to ensure consistent display of
//! prices, large numbers, and percentage changes.

/// Format a price with adaptive decimal places based on magnitude.
/// - >= $1: 2 decimal places ($87,432.15)
/// - >= $0.01: 4 decimal places ($0.5000)
/// - < $0.01: 6 decimal places ($0.000450)
pub fn format_price(price: f64) -> String {
    if price >= 1.0 {
        format!("${:.2}", price)
    } else if price >= 0.01 {
        format!("${:.4}", price)
    } else {
        format!("${:.6}", price)
    }
}

/// Format a large number with human-readable suffixes (T, B, M).
/// Prefixed with $ for monetary values.
pub fn format_large_number_usd(n: f64) -> String {
    if n >= 1_000_000_000_000.0 {
        format!("${:.2}T", n / 1_000_000_000_000.0)
    } else if n >= 1_000_000_000.0 {
        format!("${:.2}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("${:.2}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("${:.0}", n)
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
    if change_pct >= 0.0 { "📈" } else { "📉" }
}

/// Get the colored circle emoji for a change direction.
pub fn change_dot(change_pct: f64) -> &'static str {
    if change_pct >= 0.0 { "🟢" } else { "🔴" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_price_large() {
        assert_eq!(format_price(87432.15), "$87432.15");
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
        assert_eq!(format_large_number_usd(50_000.0), "$50000");
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
}
