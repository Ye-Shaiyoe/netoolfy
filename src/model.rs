//! Core data model for diagnostics: statuses, findings, check results,
//! reports, and verdicts.
//!
//! These types are the lingua franca between checks, the verdict engine,
//! and output formatters. They are `Serialize`/`Deserialize` for JSON output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Status ────────────────────────────────────────────────────────────

/// Overall status of a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Everything is healthy.
    Ok,
    /// Something is degraded but not broken.
    Warn,
    /// A definite failure was detected.
    Fail,
    /// Check was skipped (e.g. a lower layer already failed).
    Skipped,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Ok => write!(f, "ok"),
            Status::Warn => write!(f, "warn"),
            Status::Fail => write!(f, "FAIL"),
            Status::Skipped => write!(f, "skip"),
        }
    }
}

// ── Layer ─────────────────────────────────────────────────────────────

/// Network diagnostic layers, ordered bottom-up.
///
/// The layered model is fundamental: a failure at a lower layer explains
/// failures above it, so upper layers should be skipped rather than
/// producing noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// Layer 1: local network interface.
    Interface,
    /// Layer 2: default gateway reachability.
    Gateway,
    /// Layer 3: DNS resolution.
    Dns,
    /// Layer 4: general internet connectivity (ping, loss, jitter).
    Internet,
    /// Layer 5: specific target (port, TLS, HTTP timing).
    Target,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Interface => write!(f, "Interface"),
            Layer::Gateway => write!(f, "Gateway"),
            Layer::Dns => write!(f, "DNS"),
            Layer::Internet => write!(f, "Internet"),
            Layer::Target => write!(f, "Target"),
        }
    }
}

impl Layer {
    /// Return all layers in bottom-up order.
    pub fn all_ordered() -> &'static [Layer] {
        &[
            Layer::Interface,
            Layer::Gateway,
            Layer::Dns,
            Layer::Internet,
            Layer::Target,
        ]
    }
}

// ── Finding ───────────────────────────────────────────────────────────

/// A machine-readable finding code.
///
/// Each variant maps to a specific detected condition and has an
/// associated human-readable message and optional fix suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FindingCode {
    // Interface
    /// No active network interface found.
    NoInterface,
    /// Interface is up but has no IP address.
    NoIpAddress,
    /// WiFi signal is weak.
    WeakWifiSignal,

    // Gateway
    /// No default gateway detected.
    NoGateway,
    /// Gateway is unreachable.
    GatewayUnreachable,
    /// Gateway latency is high.
    GatewayLatencyHigh,

    // DNS
    /// DNS resolution is slow.
    DnsSlow,
    /// DNS resolution is failing.
    DnsFailing,
    /// DNS hijacking suspected (inconsistent answers).
    DnsHijackSuspected,

    // Internet
    /// Packet loss detected.
    PacketLoss,
    /// High jitter detected.
    HighJitter,
    /// No internet connectivity.
    NoInternet,

    // Target
    /// Target port is closed or filtered.
    PortClosed,
    /// TLS certificate is expiring soon.
    CertExpiringSoon,
    /// TLS certificate is invalid.
    CertInvalid,
    /// Target HTTP response is slow.
    HttpSlow,

    // General
    /// Everything looks good (used as a positive finding).
    AllGood,

    // Skipped
    /// Check was skipped because a lower layer failed.
    SkippedDueToLowerLayer,
}

/// A single diagnostic finding produced by a check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Machine-readable code for this finding.
    pub code: FindingCode,
    /// Human-readable description of what was found.
    pub message: String,
    /// Optional suggested fix in plain language.
    pub fix: Option<String>,
    /// Optional raw metrics (key-value pairs for JSON output).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metrics: HashMap<String, MetricValue>,
}

impl Finding {
    /// Create a new finding with the given code and message.
    pub fn new(code: FindingCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fix: None,
            metrics: HashMap::new(),
        }
    }

    /// Attach a suggested fix.
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    /// Attach a metric.
    pub fn with_metric(mut self, key: impl Into<String>, value: MetricValue) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }
}

/// A metric value that can be serialized to JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    /// An integer metric.
    Int(i64),
    /// A floating-point metric.
    Float(f64),
    /// A string metric.
    Text(String),
    /// A boolean metric.
    Bool(bool),
}

// ── CheckResult ───────────────────────────────────────────────────────

/// The result of running a single diagnostic check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Which layer this check belongs to.
    pub layer: Layer,
    /// Human-readable name of the check (e.g. "DNS").
    pub name: String,
    /// Overall status.
    pub status: Status,
    /// Detailed findings from this check.
    pub findings: Vec<Finding>,
}

impl CheckResult {
    /// Create a new check result.
    pub fn new(layer: Layer, name: impl Into<String>, status: Status) -> Self {
        Self {
            layer,
            name: name.into(),
            status,
            findings: Vec::new(),
        }
    }

    /// Add a finding to this result.
    pub fn with_finding(mut self, finding: Finding) -> Self {
        self.findings.push(finding);
        self
    }

    /// Create a "skipped" result for a check that was not run because
    /// a lower layer already failed.
    pub fn skipped(layer: Layer, name: impl Into<String>, reason: &str) -> Self {
        Self {
            layer,
            name: name.into(),
            status: Status::Skipped,
            findings: vec![Finding::new(
                FindingCode::SkippedDueToLowerLayer,
                reason.to_string(),
            )],
        }
    }
}

// ── Verdict ───────────────────────────────────────────────────────────

/// The final diagnosis: a root cause and recommended action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    /// Overall status across all checks.
    pub overall_status: Status,
    /// Plain-language summary of the root cause.
    pub summary: String,
    /// Recommended fix, if any.
    pub fix: Option<String>,
}

impl Verdict {
    /// Create a new verdict.
    pub fn new(overall_status: Status, summary: impl Into<String>) -> Self {
        Self {
            overall_status,
            summary: summary.into(),
            fix: None,
        }
    }

    /// Attach a recommended fix.
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

// ── Report ────────────────────────────────────────────────────────────

/// A complete diagnostic report: all check results plus the verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Individual check results, in the order they were run.
    pub results: Vec<CheckResult>,
    /// The overall verdict.
    pub verdict: Verdict,
}

impl Report {
    /// Create a new report from check results and a verdict.
    pub fn new(results: Vec<CheckResult>, verdict: Verdict) -> Self {
        Self { results, verdict }
    }

    /// Determine the process exit code from the verdict.
    ///
    /// - `0` = all healthy
    /// - `1` = warnings present
    /// - `2` = failures detected
    pub fn exit_code(&self) -> i32 {
        match self.verdict.overall_status {
            Status::Ok => 0,
            Status::Warn => 1,
            Status::Fail | Status::Skipped => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_ordering() {
        assert!(Status::Ok < Status::Warn);
        assert!(Status::Warn < Status::Fail);
        assert!(Status::Fail < Status::Skipped);
    }

    #[test]
    fn finding_builder() {
        let f = Finding::new(FindingCode::DnsSlow, "DNS is slow")
            .with_fix("Switch to 1.1.1.1")
            .with_metric("latency_ms", MetricValue::Int(240));
        assert_eq!(f.code, FindingCode::DnsSlow);
        assert_eq!(f.fix.as_deref(), Some("Switch to 1.1.1.1"));
        assert!(f.metrics.contains_key("latency_ms"));
    }

    #[test]
    fn check_result_skipped() {
        let r = CheckResult::skipped(Layer::Dns, "DNS", "Gateway is down");
        assert_eq!(r.status, Status::Skipped);
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].code, FindingCode::SkippedDueToLowerLayer);
    }

    #[test]
    fn report_exit_codes() {
        let ok_report = Report::new(vec![], Verdict::new(Status::Ok, "All good"));
        assert_eq!(ok_report.exit_code(), 0);

        let warn_report = Report::new(vec![], Verdict::new(Status::Warn, "Minor issues"));
        assert_eq!(warn_report.exit_code(), 1);

        let fail_report = Report::new(vec![], Verdict::new(Status::Fail, "Problems detected"));
        assert_eq!(fail_report.exit_code(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let result = CheckResult::new(Layer::Dns, "DNS", Status::Warn).with_finding(
            Finding::new(FindingCode::DnsSlow, "ISP DNS slow")
                .with_fix("Use 1.1.1.1")
                .with_metric("latency_ms", MetricValue::Int(240)),
        );
        let json = serde_json::to_string(&result).unwrap();
        let parsed: CheckResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.layer, Layer::Dns);
        assert_eq!(parsed.status, Status::Warn);
        assert_eq!(parsed.findings.len(), 1);
    }

    #[test]
    fn layer_ordering() {
        let layers = Layer::all_ordered();
        assert_eq!(layers[0], Layer::Interface);
        assert_eq!(layers[4], Layer::Target);
        for window in layers.windows(2) {
            assert!(window[0] < window[1]);
        }
    }
}
