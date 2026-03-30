//! Price alert system — get notified when an asset hits a target price.
//!
//! Alerts are stored persistently in alerts.json and checked whenever
//! the user views their portfolio or watchlist.
//!
//! Alert types:
//! - "above" — triggers when price goes above target
//! - "below" — triggers when price goes below target

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const ALERTS_FILE: &str = "alerts.json";

/// A single price alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    /// Unique alert ID (auto-incremented)
    pub id: u32,
    /// Asset symbol (e.g., "bitcoin", "AAPL")
    pub symbol: String,
    /// Alert condition: "above" or "below"
    pub condition: String,
    /// Target price
    pub target_price: f64,
    /// Optional note/reason
    pub note: String,
    /// When the alert was created
    pub created_at: String,
    /// Whether this alert has been triggered
    pub triggered: bool,
    /// When the alert was triggered (if it was)
    pub triggered_at: Option<String>,
    /// Price when triggered
    pub triggered_price: Option<f64>,
}

/// Alert manager — stores and checks price alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertManager {
    pub alerts: Vec<PriceAlert>,
    pub next_id: u32,
}

impl AlertManager {
    /// Create a new empty alert manager.
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            next_id: 1,
        }
    }

    /// Load alerts from disk, or return a new manager.
    pub fn load() -> Self {
        let path = Path::new(ALERTS_FILE);
        if !path.exists() {
            return Self::new();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    /// Save alerts to disk.
    pub fn save(&self) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(ALERTS_FILE, json).map_err(|e| format!("Write error: {}", e))
    }

    /// Add a new price alert.
    pub fn add_alert(
        &mut self,
        symbol: &str,
        condition: &str,
        target_price: f64,
        note: &str,
    ) -> Result<u32, String> {
        if condition != "above" && condition != "below" {
            return Err("Condition must be 'above' or 'below'".into());
        }
        if target_price <= 0.0 {
            return Err("Target price must be positive".into());
        }

        let now = current_timestamp();
        let id = self.next_id;
        self.next_id += 1;

        self.alerts.push(PriceAlert {
            id,
            symbol: symbol.to_string(),
            condition: condition.to_string(),
            target_price,
            note: note.to_string(),
            created_at: now,
            triggered: false,
            triggered_at: None,
            triggered_price: None,
        });

        Ok(id)
    }

    /// Remove an alert by ID.
    pub fn remove_alert(&mut self, alert_id: u32) -> bool {
        let len_before = self.alerts.len();
        self.alerts.retain(|a| a.id != alert_id);
        self.alerts.len() < len_before
    }

    /// Get all active (non-triggered) alerts.
    pub fn active_alerts(&self) -> Vec<&PriceAlert> {
        self.alerts.iter().filter(|a| !a.triggered).collect()
    }

    /// Get all triggered alerts that haven't been cleared.
    pub fn triggered_alerts(&self) -> Vec<&PriceAlert> {
        self.alerts.iter().filter(|a| a.triggered).collect()
    }

    /// Clear all triggered alerts.
    pub fn clear_triggered(&mut self) {
        self.alerts.retain(|a| !a.triggered);
    }

    /// Check alerts against current prices.
    /// Returns a list of newly triggered alerts.
    pub fn check_alerts(
        &mut self,
        price_map: &HashMap<String, f64>,
    ) -> Vec<(u32, String, String, f64, f64)> {
        // Returns: (alert_id, symbol, condition, target_price, current_price)
        let mut triggered = Vec::new();

        for alert in self.alerts.iter_mut() {
            if alert.triggered {
                continue;
            }
            if let Some(&current_price) = price_map.get(&alert.symbol) {
                let is_triggered = match alert.condition.as_str() {
                    "above" => current_price >= alert.target_price,
                    "below" => current_price <= alert.target_price,
                    _ => false,
                };

                if is_triggered {
                    alert.triggered = true;
                    alert.triggered_at = Some(current_timestamp());
                    alert.triggered_price = Some(current_price);
                    triggered.push((
                        alert.id,
                        alert.symbol.clone(),
                        alert.condition.clone(),
                        alert.target_price,
                        current_price,
                    ));
                }
            }
        }

        triggered
    }

    /// Format alerts for display.
    pub fn format_alerts(&self) -> String {
        let active = self.active_alerts();
        let triggered = self.triggered_alerts();

        let mut output = String::new();
        output.push_str("🔔 Price Alerts\n");
        output.push_str("─────────────────────────────────────────\n");

        if active.is_empty() && triggered.is_empty() {
            output.push_str("  No alerts set.\n");
            output.push_str("  Add one with: /alert bitcoin below 80000\n");
            output.push_str("─────────────────────────────────────────\n");
            return output;
        }

        if !active.is_empty() {
            output.push_str(&format!("  📌 Active Alerts ({}):\n", active.len()));
            for alert in &active {
                let arrow = if alert.condition == "above" {
                    "↑"
                } else {
                    "↓"
                };
                output.push_str(&format!(
                    "    #{} {} {} {} ${:.2}",
                    alert.id, alert.symbol, arrow, alert.condition, alert.target_price,
                ));
                if !alert.note.is_empty() {
                    output.push_str(&format!(" — {}", alert.note));
                }
                output.push('\n');
            }
        }

        if !triggered.is_empty() {
            output.push_str(&format!("\n  ⚡ Triggered ({}):\n", triggered.len()));
            for alert in &triggered {
                let price_str = alert
                    .triggered_price
                    .map(|p| format!("${:.2}", p))
                    .unwrap_or_else(|| "?".to_string());
                output.push_str(&format!(
                    "    #{} {} {} ${:.2} — hit at {}\n",
                    alert.id, alert.symbol, alert.condition, alert.target_price, price_str,
                ));
            }
            output.push_str("  Use /alert clear to dismiss triggered alerts.\n");
        }

        output.push_str("─────────────────────────────────────────\n");
        output
    }

    /// Get unique symbols from active alerts for price fetching.
    pub fn active_symbols(&self) -> Vec<String> {
        let active = self.active_alerts();
        let mut symbols: Vec<String> = active.iter().map(|a| a.symbol.clone()).collect();
        symbols.sort();
        symbols.dedup();
        symbols
    }
}

/// Get current timestamp (delegates to shared format module).
fn current_timestamp() -> String {
    super::format::current_timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_alert_manager() {
        let am = AlertManager::new();
        assert!(am.alerts.is_empty());
        assert_eq!(am.next_id, 1);
    }

    #[test]
    fn test_add_alert() {
        let mut am = AlertManager::new();
        let id = am
            .add_alert("bitcoin", "below", 80000.0, "Buy the dip")
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(am.alerts.len(), 1);
        assert_eq!(am.alerts[0].symbol, "bitcoin");
        assert_eq!(am.alerts[0].condition, "below");
        assert_eq!(am.alerts[0].target_price, 80000.0);
        assert!(!am.alerts[0].triggered);
    }

    #[test]
    fn test_add_alert_invalid_condition() {
        let mut am = AlertManager::new();
        assert!(am.add_alert("bitcoin", "maybe", 80000.0, "").is_err());
    }

    #[test]
    fn test_add_alert_invalid_price() {
        let mut am = AlertManager::new();
        assert!(am.add_alert("bitcoin", "below", 0.0, "").is_err());
        assert!(am.add_alert("bitcoin", "below", -100.0, "").is_err());
    }

    #[test]
    fn test_remove_alert() {
        let mut am = AlertManager::new();
        let id = am.add_alert("bitcoin", "below", 80000.0, "").unwrap();
        assert!(am.remove_alert(id));
        assert!(am.alerts.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_alert() {
        let mut am = AlertManager::new();
        assert!(!am.remove_alert(999));
    }

    #[test]
    fn test_check_alerts_below_triggered() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "Buy signal")
            .unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 79000.0);

        let triggered = am.check_alerts(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].1, "bitcoin");
        assert_eq!(triggered[0].2, "below");
        assert!(am.alerts[0].triggered);
    }

    #[test]
    fn test_check_alerts_above_triggered() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "above", 100000.0, "Moon alert")
            .unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 101000.0);

        let triggered = am.check_alerts(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].2, "above");
    }

    #[test]
    fn test_check_alerts_not_triggered() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "").unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 85000.0);

        let triggered = am.check_alerts(&prices);
        assert!(triggered.is_empty());
        assert!(!am.alerts[0].triggered);
    }

    #[test]
    fn test_check_alerts_already_triggered() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "").unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 79000.0);

        // Trigger it
        am.check_alerts(&prices);
        assert!(am.alerts[0].triggered);

        // Check again — should NOT re-trigger
        let triggered = am.check_alerts(&prices);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_active_and_triggered_alerts() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "").unwrap();
        am.add_alert("AAPL", "above", 200.0, "").unwrap();

        assert_eq!(am.active_alerts().len(), 2);
        assert_eq!(am.triggered_alerts().len(), 0);

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 79000.0);
        am.check_alerts(&prices);

        assert_eq!(am.active_alerts().len(), 1);
        assert_eq!(am.triggered_alerts().len(), 1);
    }

    #[test]
    fn test_clear_triggered() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "").unwrap();
        am.add_alert("AAPL", "above", 200.0, "").unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 79000.0);
        am.check_alerts(&prices);

        am.clear_triggered();
        assert_eq!(am.alerts.len(), 1); // Only AAPL remains
        assert_eq!(am.alerts[0].symbol, "AAPL");
    }

    #[test]
    fn test_active_symbols() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "").unwrap();
        am.add_alert("bitcoin", "above", 100000.0, "").unwrap();
        am.add_alert("AAPL", "above", 200.0, "").unwrap();

        let symbols = am.active_symbols();
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains(&"AAPL".to_string()));
        assert!(symbols.contains(&"bitcoin".to_string()));
    }

    #[test]
    fn test_format_alerts_empty() {
        let am = AlertManager::new();
        let output = am.format_alerts();
        assert!(output.contains("No alerts set"));
    }

    #[test]
    fn test_format_alerts_with_data() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "Buy the dip")
            .unwrap();
        am.add_alert("AAPL", "above", 200.0, "").unwrap();

        let output = am.format_alerts();
        assert!(output.contains("Active Alerts (2)"));
        assert!(output.contains("bitcoin"));
        assert!(output.contains("AAPL"));
        assert!(output.contains("Buy the dip"));
    }

    #[test]
    fn test_multiple_alerts_same_symbol() {
        let mut am = AlertManager::new();
        am.add_alert("bitcoin", "below", 80000.0, "First level")
            .unwrap();
        am.add_alert("bitcoin", "below", 70000.0, "Second level")
            .unwrap();

        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), 75000.0);

        let triggered = am.check_alerts(&prices);
        assert_eq!(triggered.len(), 1); // Only the 80k alert triggers
        assert_eq!(am.active_alerts().len(), 1); // 70k still active
    }
}
