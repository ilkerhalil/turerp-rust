//! Alert duration tracker for sustained-threshold alerting
//!
//! Prevents flapping alerts by requiring a metric to breach a threshold
//! for a configurable `duration_sec` before firing.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Tracks sustained threshold breaches for alert rules.
///
/// Each entry is keyed by `(rule_id, metric_label_key)` and stores
/// the timestamp when the breach first began and the last observed value.
///
/// Only fires an alert if the condition has been met continuously
/// for at least `rule.duration_sec`.
#[derive(Clone, Debug)]
pub struct AlertDurationTracker {
    breaches: HashMap<(String, String), (DateTime<Utc>, f64)>,
}

impl AlertDurationTracker {
    pub fn new() -> Self {
        Self {
            breaches: HashMap::new(),
        }
    }

    /// Record an evaluation result for a rule + metric combination.
    ///
    /// Returns `Some((first_breach_time, last_value))` if the breach has
    /// been sustained for at least `duration_sec`, meaning an alert
    /// should be fired. Returns `None` if no alert should fire.
    ///
    /// If `condition_met` is `false`, any existing breach state is cleared.
    pub fn record(
        &mut self,
        rule_id: &str,
        metric_key: &str,
        condition_met: bool,
        current_value: f64,
        duration_sec: i64,
    ) -> Option<(DateTime<Utc>, f64)> {
        let key = (rule_id.to_string(), metric_key.to_string());

        if !condition_met {
            // Condition cleared — remove any pending breach
            self.breaches.remove(&key);
            return None;
        }

        let now = Utc::now();

        if let Some((first_breach, last_val)) = self.breaches.get_mut(&key) {
            // Already breaching — update last value
            *last_val = current_value;
            let elapsed = now.signed_duration_since(*first_breach).num_seconds();
            if elapsed >= duration_sec {
                return Some((*first_breach, *last_val));
            }
        } else {
            // First time this rule+metric has breached
            self.breaches.insert(key.clone(), (now, current_value));
        }

        None
    }

    /// Check if a given rule+metric is currently in breach (regardless
    /// of whether the duration threshold has been met).
    pub fn is_breaching(&self, rule_id: &str, metric_key: &str) -> bool {
        self.breaches
            .contains_key(&(rule_id.to_string(), metric_key.to_string()))
    }

    /// Get the number of active (unresolved) breaches being tracked.
    pub fn active_breach_count(&self) -> usize {
        self.breaches.len()
    }

    /// Clear all tracked breach state (e.g. on shutdown or test reset).
    pub fn clear(&mut self) {
        self.breaches.clear();
    }
}

impl Default for AlertDurationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_record_first_breach_not_fired() {
        let mut tracker = AlertDurationTracker::new();
        let result = tracker.record("rule-1", "cpu", true, 85.0, 60);
        assert!(result.is_none());
        assert!(tracker.is_breaching("rule-1", "cpu"));
    }

    #[test]
    fn test_record_condition_clears_breach() {
        let mut tracker = AlertDurationTracker::new();
        tracker.record("rule-1", "cpu", true, 85.0, 60);
        assert!(tracker.is_breaching("rule-1", "cpu"));

        let result = tracker.record("rule-1", "cpu", false, 40.0, 60);
        assert!(result.is_none());
        assert!(!tracker.is_breaching("rule-1", "cpu"));
    }

    #[test]
    fn test_record_sustained_breach_fires() {
        let mut tracker = AlertDurationTracker::new();
        // First record: not enough time
        let result = tracker.record("rule-1", "cpu", true, 85.0, 2);
        assert!(result.is_none());

        // Wait for duration to pass
        thread::sleep(Duration::from_secs(3));

        // Second record: now fires
        let result = tracker.record("rule-1", "cpu", true, 90.0, 2);
        assert!(result.is_some());
        let (first_breach, last_value) = result.unwrap();
        assert!(last_value >= 85.0);
        assert!(Utc::now().signed_duration_since(first_breach).num_seconds() >= 2);
    }

    #[test]
    fn test_different_rules_are_isolated() {
        let mut tracker = AlertDurationTracker::new();
        tracker.record("rule-1", "cpu", true, 85.0, 60);
        assert!(tracker.is_breaching("rule-1", "cpu"));
        assert!(!tracker.is_breaching("rule-2", "cpu"));
        assert!(!tracker.is_breaching("rule-1", "memory"));
    }

    #[test]
    fn test_active_breach_count() {
        let mut tracker = AlertDurationTracker::new();
        assert_eq!(tracker.active_breach_count(), 0);
        tracker.record("rule-1", "cpu", true, 85.0, 60);
        assert_eq!(tracker.active_breach_count(), 1);
        tracker.record("rule-2", "memory", true, 95.0, 60);
        assert_eq!(tracker.active_breach_count(), 2);
        tracker.record("rule-1", "cpu", false, 40.0, 60);
        assert_eq!(tracker.active_breach_count(), 1);
    }

    #[test]
    fn test_clear() {
        let mut tracker = AlertDurationTracker::new();
        tracker.record("rule-1", "cpu", true, 85.0, 60);
        tracker.record("rule-2", "memory", true, 95.0, 60);
        assert_eq!(tracker.active_breach_count(), 2);
        tracker.clear();
        assert_eq!(tracker.active_breach_count(), 0);
    }
}
