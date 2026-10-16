//! Per-device performance metrics, computed by the device thread over a rolling window.
//! JSON fields match the former Python service (`GET /metrics`, `/ws/metrics`).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::engine::ExchangeStats;

/// Window length (counts reset after it, like the Python service).
pub const WINDOW: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Metrics {
    device_id: String,
    transport: &'static str,
    window_start: Instant,
    start_stats: ExchangeStats,
    last_stats: ExchangeStats,
    api_requests: u64,
    api_latency_ms: VecDeque<f64>,
    poll_ms: VecDeque<f64>,
    queue_depth: VecDeque<f64>,
    previous_quality: Option<f64>,
}

fn push(q: &mut VecDeque<f64>, v: f64, cap: usize) {
    if q.len() == cap {
        q.pop_front();
    }
    q.push_back(v);
}

fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let i = ((sorted.len() as f64) * p) as usize;
    Some(sorted[i.min(sorted.len() - 1)])
}

fn mean(q: &VecDeque<f64>) -> f64 {
    if q.is_empty() { 0.0 } else { q.iter().sum::<f64>() / q.len() as f64 }
}

impl Metrics {
    pub fn new(device_id: &str, transport: &'static str) -> Self {
        Metrics {
            device_id: device_id.to_string(),
            transport,
            window_start: Instant::now(),
            start_stats: ExchangeStats::default(),
            last_stats: ExchangeStats::default(),
            api_requests: 0,
            api_latency_ms: VecDeque::new(),
            poll_ms: VecDeque::new(),
            queue_depth: VecDeque::new(),
            previous_quality: None,
        }
    }

    /// A new connection: engine counters restart from zero.
    pub fn connected(&mut self) {
        self.start_stats = ExchangeStats::default();
        self.last_stats = ExchangeStats::default();
    }

    pub fn update_stats(&mut self, s: ExchangeStats) {
        self.last_stats = s;
    }

    pub fn api_request(&mut self, latency: Duration) {
        self.api_requests += 1;
        push(&mut self.api_latency_ms, latency.as_secs_f64() * 1000.0, 1000);
    }

    pub fn poll(&mut self, d: Duration) {
        push(&mut self.poll_ms, d.as_secs_f64() * 1000.0, 100);
    }

    pub fn queue_sample(&mut self, depth: usize) {
        push(&mut self.queue_depth, depth as f64, 100);
    }

    fn quality(&self, ops: u64, fails: u64) -> f64 {
        if ops == 0 { 50.0 } else { 100.0 * (ops - fails.min(ops)) as f64 / ops as f64 }
    }

    pub fn snapshot(&self) -> Value {
        let window = self.window_start.elapsed().as_secs_f64().max(1e-3);
        let ops = self.last_stats.exchanges.saturating_sub(self.start_stats.exchanges);
        let fails = self.last_stats.failures.saturating_sub(self.start_stats.failures);
        let busy = self.last_stats.busy.saturating_sub(self.start_stats.busy).as_secs_f64();
        let mut lat: Vec<f64> = self.api_latency_ms.iter().copied().collect();
        lat.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let quality = self.quality(ops, fails);
        let tier = match quality {
            q if ops == 0 && q == 50.0 => "unknown",
            q if q >= 90.0 => "excellent",
            q if q >= 75.0 => "good",
            q if q >= 50.0 => "fair",
            q if q >= 25.0 => "poor",
            _ => "critical",
        };
        let trend = match self.previous_quality {
            Some(p) if quality > p + 5.0 => "improving",
            Some(p) if quality < p - 5.0 => "degrading",
            _ => "stable",
        };
        json!({
            "device_id": self.device_id,
            "window_duration_s": window,
            "utilization_pct": (100.0 * busy / window).min(100.0),
            "qps": ops as f64 / window,
            "api_request_count": self.api_requests,
            "api_latency_p95_ms": percentile(&lat, 0.95),
            "api_latency_p99_ms": percentile(&lat, 0.99),
            "avg_queue_depth": mean(&self.queue_depth),
            "avg_poll_duration_ms": mean(&self.poll_ms),
            "total_operations": ops,
            "throttle_events": 0,
            "throttle_rate_pct": 0.0,
            "backoff_multiplier": 1.0,
            "quality_score": quality,
            "quality_tier": tier,
            "quality_trend": trend,
            "transport_type": self.transport,
        })
    }

    /// Start a new window when the current one is over.
    pub fn roll_window(&mut self) {
        if self.window_start.elapsed() >= WINDOW {
            let ops = self.last_stats.exchanges.saturating_sub(self.start_stats.exchanges);
            let fails = self.last_stats.failures.saturating_sub(self.start_stats.failures);
            self.previous_quality = Some(self.quality(ops, fails));
            self.window_start = Instant::now();
            self.start_stats = self.last_stats;
            self.api_requests = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_fields_and_rates() {
        let mut m = Metrics::new("t1", "serial");
        let s0 = m.snapshot();
        assert_eq!(s0["quality_tier"], "unknown");
        assert_eq!(s0["api_latency_p95_ms"], Value::Null);
        m.update_stats(ExchangeStats { exchanges: 10, failures: 1, busy: Duration::from_millis(5) });
        m.api_request(Duration::from_millis(20));
        m.api_request(Duration::from_millis(40));
        m.poll(Duration::from_millis(100));
        m.queue_sample(2);
        let s = m.snapshot();
        assert_eq!(s["total_operations"], 10);
        assert_eq!(s["api_request_count"], 2);
        assert_eq!(s["api_latency_p99_ms"], 40.0);
        assert_eq!(s["avg_poll_duration_ms"], 100.0);
        assert_eq!(s["avg_queue_depth"], 2.0);
        assert_eq!(s["quality_score"], 90.0);
        assert_eq!(s["quality_tier"], "excellent");
        for k in ["utilization_pct", "qps", "window_duration_s", "throttle_rate_pct", "backoff_multiplier"] {
            assert!(s[k].is_number(), "{k} must be a number for the UI's toFixed()");
        }
    }
}
