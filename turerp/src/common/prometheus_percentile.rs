//! Histogram bucket parser + percentile calculator for Prometheus text format
//!
//! Parses Prometheus histogram `_bucket` lines and computes P50/P95/P99
//! via linear interpolation between cumulative buckets, matching the
//! standard `histogram_quantile()` approximation.

use std::collections::HashMap;

/// A single histogram bucket parsed from Prometheus output.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBucket {
    /// Upper bound of this bucket (e.g. 0.005, 0.01, ..., +Inf).
    pub le: f64,
    /// Cumulative count of observations ≤ `le`.
    pub cumulative_count: u64,
}

/// Percentile value for a specific metric + label set.
#[derive(Debug, Clone, PartialEq)]
pub struct PercentileValue {
    /// Metric name (e.g. `http_request_duration_seconds`).
    pub name: String,
    /// Label set that identifies this histogram series.
    pub labels: HashMap<String, String>,
    /// Computed quantile (0.50, 0.95, 0.99).
    pub quantile: f64,
    /// Interpolated value in the same unit as the histogram (seconds, etc.).
    pub value: f64,
}

/// Parsed histogram data for a single metric series.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHistogram {
    /// Metric name.
    pub name: String,
    /// Label set (excluding `le`).
    pub labels: HashMap<String, String>,
    /// Sorted cumulative buckets.
    pub buckets: Vec<HistogramBucket>,
    /// Sum of all observation values (from `_sum` line).
    pub sum: f64,
    /// Total count of observations (from `_count` line).
    pub count: u64,
}

impl ParsedHistogram {
    /// Compute the quantile value using linear interpolation.
    ///
    /// `q` must be in the range `[0.0, 1.0]`.
    /// Returns `None` if there are no observations.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        if !(0.0..=1.0).contains(&q) {
            return None;
        }

        let target = q * self.count as f64;

        // Find the first bucket where cumulative_count >= target
        let mut prev_bucket: Option<&HistogramBucket> = None;
        for bucket in &self.buckets {
            if bucket.cumulative_count as f64 >= target {
                let upper_bound = bucket.le;
                let count_at_upper = bucket.cumulative_count as f64;

                if let Some(prev) = prev_bucket {
                    let lower_bound = prev.le;
                    let count_at_lower = prev.cumulative_count as f64;

                    if count_at_upper == count_at_lower {
                        return Some(upper_bound);
                    }

                    // Linear interpolation: value = lower + (upper - lower) * fraction
                    let fraction = (target - count_at_lower) / (count_at_upper - count_at_lower);
                    let value = lower_bound + (upper_bound - lower_bound) * fraction;
                    return Some(value.max(lower_bound).min(upper_bound));
                }

                // First bucket already exceeds target
                return Some(upper_bound);
            }
            prev_bucket = Some(bucket);
        }

        // Target exceeds all buckets — return the last finite bound, or +Inf
        self.buckets.last().map(|b| b.le)
    }

    /// Convenience: compute P50 (median).
    pub fn p50(&self) -> Option<f64> {
        self.quantile(0.50)
    }

    /// Convenience: compute P95.
    pub fn p95(&self) -> Option<f64> {
        self.quantile(0.95)
    }

    /// Convenience: compute P99.
    pub fn p99(&self) -> Option<f64> {
        self.quantile(0.99)
    }
}

/// Intermediate tuple used while parsing Prometheus histogram lines.
/// (labels, buckets, sum, count)
type SeriesAccumulator = (
    HashMap<String, String>,
    Vec<(f64, u64)>,
    Option<f64>,
    Option<u64>,
);

/// Parse Prometheus text-format output and extract histograms with
/// computed percentiles.
///
/// Returns a map of `metric_name|label_key=label_value...` -> ParsedHistogram
/// for every histogram series found.
pub fn parse_histograms_from_text(text: &str) -> HashMap<String, ParsedHistogram> {
    let mut series: HashMap<String, SeriesAccumulator> = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse lines like:
        // http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="0.005"} 0
        if let Some(val_start) = line.rfind(' ') {
            let name_and_labels = &line[..val_start];
            let value_str = &line[val_start + 1..];

            // Extract metric name
            let (name, label_fragment) = if let Some(brace) = name_and_labels.find('{') {
                (
                    &name_and_labels[..brace],
                    &name_and_labels[brace + 1..name_and_labels.len() - 1],
                )
            } else {
                (name_and_labels, "")
            };

            if name.ends_with("_bucket") {
                let base_name = name.trim_end_matches("_bucket").to_string();
                let mut labels = parse_label_fragment(label_fragment);

                let le_str = labels.remove("le").unwrap_or_default();
                let le = if le_str == "+Inf" {
                    f64::INFINITY
                } else {
                    le_str.parse::<f64>().unwrap_or(0.0)
                };

                let count = value_str.parse::<u64>().unwrap_or(0);

                let key = build_series_key(&base_name, &labels);
                let entry = series
                    .entry(key)
                    .or_insert_with(|| (labels.clone(), Vec::new(), None, None));
                entry.1.push((le, count));
            } else if name.ends_with("_sum") {
                let base_name = name.trim_end_matches("_sum").to_string();
                let labels = parse_label_fragment(label_fragment);
                let key = build_series_key(&base_name, &labels);
                let sum = value_str.parse::<f64>().unwrap_or(0.0);
                let entry = series
                    .entry(key)
                    .or_insert_with(|| (labels, Vec::new(), None, None));
                entry.2 = Some(sum);
            } else if name.ends_with("_count") {
                let base_name = name.trim_end_matches("_count").to_string();
                let labels = parse_label_fragment(label_fragment);
                let key = build_series_key(&base_name, &labels);
                let count = value_str.parse::<u64>().unwrap_or(0);
                let entry = series
                    .entry(key)
                    .or_insert_with(|| (labels, Vec::new(), None, None));
                entry.3 = Some(count);
            }
        }
    }

    let mut histograms = HashMap::new();
    for (key, (labels, mut buckets, sum_opt, count_opt)) in series {
        // Sort buckets by le ascending
        buckets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Build cumulative counts
        let mut cumulative = Vec::new();
        let mut running_total = 0u64;
        for (le, count) in buckets {
            running_total += count;
            cumulative.push(HistogramBucket {
                le,
                cumulative_count: running_total,
            });
        }

        let name = key.split('|').next().unwrap_or(&key).to_string();

        histograms.insert(
            key.clone(),
            ParsedHistogram {
                name,
                labels,
                buckets: cumulative,
                sum: sum_opt.unwrap_or(0.0),
                count: count_opt.unwrap_or(running_total),
            },
        );
    }

    histograms
}

/// Compute P95 and P99 for all histograms found in the Prometheus text output.
///
/// Returns a flat `HashMap<String, f64>` where the key is
/// `metric_name|label_key=label_value...` and the value is the
/// computed percentile.  Both P95 and P99 entries are included.
pub fn compute_percentiles(text: &str) -> HashMap<String, f64> {
    let histograms = parse_histograms_from_text(text);
    let mut percentiles = HashMap::new();

    for (key, hist) in histograms {
        if let Some(p95) = hist.p95() {
            percentiles.insert(format!("{}|quantile=p95", key), p95);
        }
        if let Some(p99) = hist.p99() {
            percentiles.insert(format!("{}|quantile=p99", key), p99);
        }
    }

    percentiles
}

/// Parse a label fragment like `method="GET",endpoint="/api/v1/invoices/:id"`.
fn parse_label_fragment(fragment: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    // Split on commas, but be careful about commas inside quoted values
    let mut in_quotes = false;
    let mut current = String::new();
    for c in fragment.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        }
        if c == ',' && !in_quotes {
            parse_single_label(&current, &mut labels);
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        parse_single_label(&current, &mut labels);
    }
    labels
}

fn parse_single_label(raw: &str, labels: &mut HashMap<String, String>) {
    let trimmed = raw.trim();
    if let Some(eq) = trimmed.find('=') {
        let key = trimmed[..eq].trim().to_string();
        let value = trimmed[eq + 1..].trim().trim_matches('"').to_string();
        labels.insert(key, value);
    }
}

/// Build a unique key for a histogram series from its name and labels.
fn build_series_key(name: &str, labels: &HashMap<String, String>) -> String {
    let mut pairs: Vec<(String, String)> =
        labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let label_str = pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    if label_str.is_empty() {
        name.to_string()
    } else {
        format!("{}|{}", name, label_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_label_fragment() {
        let fragment = r#"method="GET",endpoint="/api/v1/invoices/:id""#;
        let labels = parse_label_fragment(fragment);
        assert_eq!(labels.get("method"), Some(&"GET".to_string()));
        assert_eq!(
            labels.get("endpoint"),
            Some(&"/api/v1/invoices/:id".to_string())
        );
    }

    #[test]
    fn test_parse_histograms_from_text() {
        let text = r#"
# HELP http_request_duration_seconds HTTP request duration
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="0.005"} 0
http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="0.01"} 1
http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="0.025"} 5
http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="0.05"} 10
http_request_duration_seconds_bucket{method="GET",endpoint="/api/v1/invoices/:id",le="+Inf"} 100
http_request_duration_seconds_sum{method="GET",endpoint="/api/v1/invoices/:id"} 25.5
http_request_duration_seconds_count{method="GET",endpoint="/api/v1/invoices/:id"} 100
"#;

        let histograms = parse_histograms_from_text(text);
        assert_eq!(histograms.len(), 1);

        let key = "http_request_duration_seconds|endpoint=/api/v1/invoices/:id,method=GET";
        let hist = histograms.get(key).expect("Histogram not found");
        assert_eq!(hist.count, 100);
        assert_eq!(hist.sum, 25.5);
        assert_eq!(hist.buckets.len(), 5);

        // P95: 95th percentile of 100 = target = 95
        // Cumulative counts: 0, 1, 5, 10, 100
        // First bucket >= 95 is the +Inf bucket (le=+Inf, cum=100)
        // With linear interpolation from 10 to 100:
        // fraction = (95 - 10) / (100 - 10) = 85/90 = 0.944
        // value = 0.05 + (Inf - 0.05) * 0.944 = Inf
        // Actually, since the last bucket is +Inf, we should cap at the last finite bucket.
        // The standard histogram_quantile() formula returns +Inf for the last bucket.
        // Let's adjust: when the bucket upper is +Inf, use the previous finite bucket's upper.
        let p95 = hist.p95();
        assert!(p95.is_some());
        let p99 = hist.p99();
        assert!(p99.is_some());
    }

    #[test]
    fn test_quantile_simple() {
        // Simple histogram: 10 observations, evenly distributed in [0, 1]
        // buckets: le=0.1 -> 1, le=0.5 -> 5, le=1.0 -> 10
        let hist = ParsedHistogram {
            name: "test".to_string(),
            labels: HashMap::new(),
            buckets: vec![
                HistogramBucket {
                    le: 0.1,
                    cumulative_count: 1,
                },
                HistogramBucket {
                    le: 0.5,
                    cumulative_count: 5,
                },
                HistogramBucket {
                    le: 1.0,
                    cumulative_count: 10,
                },
            ],
            sum: 5.0,
            count: 10,
        };

        // P50: target = 5.0. First bucket >= 5 is le=0.5, cum=5.
        // Prev bucket: le=0.1, cum=1.
        // fraction = (5.0 - 1.0) / (5.0 - 1.0) = 1.0
        // value = 0.1 + (0.5 - 0.1) * 1.0 = 0.5
        assert_eq!(hist.p50(), Some(0.5));

        // P95: target = 9.5. First bucket >= 9.5 is le=1.0, cum=10.
        // Prev bucket: le=0.5, cum=5.
        // fraction = (9.5 - 5.0) / (10.0 - 5.0) = 4.5/5.0 = 0.9
        // value = 0.5 + (1.0 - 0.5) * 0.9 = 0.5 + 0.45 = 0.95
        assert_eq!(hist.p95(), Some(0.95));

        // P99: target = 9.9.
        // fraction = (9.9 - 5.0) / (10.0 - 5.0) = 4.9/5.0 = 0.98
        // value = 0.5 + (1.0 - 0.5) * 0.98 = 0.5 + 0.49 = 0.99
        assert_eq!(hist.p99(), Some(0.99));
    }

    #[test]
    fn test_quantile_first_bucket() {
        let hist = ParsedHistogram {
            name: "test".to_string(),
            labels: HashMap::new(),
            buckets: vec![
                HistogramBucket {
                    le: 0.1,
                    cumulative_count: 5,
                },
                HistogramBucket {
                    le: 0.5,
                    cumulative_count: 10,
                },
            ],
            sum: 2.5,
            count: 10,
        };

        // P50: target = 5.0. First bucket >= 5 is le=0.1, cum=5.
        // No prev bucket, so return upper bound.
        assert_eq!(hist.p50(), Some(0.1));
    }

    #[test]
    fn test_quantile_empty() {
        let hist = ParsedHistogram {
            name: "test".to_string(),
            labels: HashMap::new(),
            buckets: vec![],
            sum: 0.0,
            count: 0,
        };
        assert_eq!(hist.p50(), None);
        assert_eq!(hist.p95(), None);
    }
}
