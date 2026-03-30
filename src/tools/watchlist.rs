//! Watchlist management — persistent list of tracked assets.
//!
//! Stored as a simple JSON file (watchlist.json) in the current directory.
//! Users can add/remove/view their watched assets and get quick price checks.

use std::collections::BTreeSet;
use std::path::Path;

const WATCHLIST_FILE: &str = "watchlist.json";

/// A user's watchlist of asset symbols.
#[derive(Debug, Clone)]
pub struct Watchlist {
    pub symbols: BTreeSet<String>,
}

impl Watchlist {
    /// Load watchlist from disk, or return empty if file doesn't exist.
    pub fn load() -> Self {
        let path = Path::new(WATCHLIST_FILE);
        if !path.exists() {
            return Self {
                symbols: BTreeSet::new(),
            };
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let symbols: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
                Self {
                    symbols: symbols.into_iter().collect(),
                }
            }
            Err(_) => Self {
                symbols: BTreeSet::new(),
            },
        }
    }

    /// Save watchlist to disk.
    pub fn save(&self) -> Result<(), String> {
        let symbols: Vec<&String> = self.symbols.iter().collect();
        let json =
            serde_json::to_string_pretty(&symbols).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(WATCHLIST_FILE, json).map_err(|e| format!("Write error: {}", e))
    }

    /// Add a symbol to the watchlist. Returns true if it was new.
    pub fn add(&mut self, symbol: &str) -> bool {
        let normalized = normalize_symbol(symbol);
        self.symbols.insert(normalized)
    }

    /// Remove a symbol from the watchlist. Returns true if it was present.
    pub fn remove(&mut self, symbol: &str) -> bool {
        let normalized = normalize_symbol(symbol);
        self.symbols.remove(&normalized)
    }

    /// Check if a symbol is in the watchlist.
    pub fn contains(&self, symbol: &str) -> bool {
        let normalized = normalize_symbol(symbol);
        self.symbols.contains(&normalized)
    }

    /// Check if the watchlist is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Get the number of watched symbols.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }
}

/// Normalize a symbol for consistent storage.
/// Crypto IDs (lowercase words like "bitcoin") stay lowercase.
/// Stock tickers (uppercase like "AAPL") stay uppercase.
fn normalize_symbol(symbol: &str) -> String {
    let s = symbol.trim();
    // If it looks like a stock ticker, uppercase it
    if s.starts_with('^')
        || s.contains('.')
        || s.contains('-')
        || (s.len() <= 5 && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
    {
        s.to_uppercase()
    } else {
        s.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_crypto() {
        assert_eq!(normalize_symbol("bitcoin"), "bitcoin");
        assert_eq!(normalize_symbol("Bitcoin"), "bitcoin");
        assert_eq!(normalize_symbol("ethereum"), "ethereum");
    }

    #[test]
    fn test_normalize_stock() {
        assert_eq!(normalize_symbol("AAPL"), "AAPL");
        assert_eq!(normalize_symbol("^GSPC"), "^GSPC");
        assert_eq!(normalize_symbol("BTC-USD"), "BTC-USD");
        assert_eq!(normalize_symbol("BRK.B"), "BRK.B");
    }

    #[test]
    fn test_watchlist_add_remove() {
        let mut wl = Watchlist {
            symbols: BTreeSet::new(),
        };
        assert!(wl.add("bitcoin"));
        assert!(!wl.add("bitcoin")); // duplicate
        assert!(wl.contains("bitcoin"));
        assert_eq!(wl.len(), 1);

        assert!(wl.add("AAPL"));
        assert_eq!(wl.len(), 2);

        assert!(wl.remove("bitcoin"));
        assert!(!wl.remove("bitcoin")); // already removed
        assert!(!wl.contains("bitcoin"));
        assert_eq!(wl.len(), 1);
    }

    #[test]
    fn test_watchlist_empty() {
        let wl = Watchlist {
            symbols: BTreeSet::new(),
        };
        assert!(wl.is_empty());
        assert_eq!(wl.len(), 0);
    }

    #[test]
    fn test_watchlist_case_normalization() {
        let mut wl = Watchlist {
            symbols: BTreeSet::new(),
        };
        wl.add("Bitcoin");
        assert!(wl.contains("bitcoin"));
        assert!(wl.contains("Bitcoin"));
    }
}
