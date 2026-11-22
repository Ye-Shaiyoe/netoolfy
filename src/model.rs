use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Interface,
    Gateway,
    Dns,
    Internet,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FindingCode {
    NoInterface,
    NoIpAddress,
    WeakWifiSignal,
    NoGateway,
    GatewayUnreachable,
    GatewayLatencyHigh,
    DnsSlow,
    DnsFailing,
    DnsHijackSuspected,
    PacketLoss,
    HighJitter,
    NoInternet,
    PortClosed,
    CertExpiringSoon,
    CertInvalid,
    HttpSlow,
    AllGood,
    SkippedDueToLowerLayer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub code: FindingCode,
    pub message: String,
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metrics: HashMap<String, MetricValue>,
}

impl Finding {
    pub fn new(code: FindingCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fix: None,
            metrics: HashMap::new(),
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    pub fn with_metric(mut self, key: impl Into<String>, value: MetricValue) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub layer: Layer,
    pub name: String,
    pub status: Status,
    pub findings: Vec<Finding>,
}

impl CheckResult {
    pub fn new(layer: Layer, name: impl Into<String>, status: Status) -> Self {
        Self {
            layer,
            name: name.into(),
            status,
            findings: Vec::new(),
        }
    }

    pub fn with_finding(mut self, finding: Finding) -> Self {
        self.findings.push(finding);
        self
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub overall_status: Status,
    pub summary: String,
    pub fix: Option<String>,
}

impl Verdict {
    pub fn new(overall_status: Status, summary: impl Into<String>) -> Self {
        Self {
            overall_status,
            summary: summary.into(),
            fix: None,
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub results: Vec<CheckResult>,
    pub verdict: Verdict,
}

impl Report {
    pub fn new(results: Vec<CheckResult>, verdict: Verdict) -> Self {
        Self { results, verdict }
    }

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
    fn orders_statuses_by_severity() {
        assert!(Status::Ok < Status::Warn);
        assert!(Status::Warn < Status::Fail);
        assert!(Status::Fail < Status::Skipped);
    }

    #[test]
    fn builds_finding_with_fix_and_metrics() {
        let finding = Finding::new(FindingCode::DnsSlow, "DNS is slow")
            .with_fix("Switch to 1.1.1.1")
            .with_metric("latency_ms", MetricValue::Int(240));

        assert_eq!(finding.code, FindingCode::DnsSlow);
        assert_eq!(finding.fix.as_deref(), Some("Switch to 1.1.1.1"));
        assert!(finding.metrics.contains_key("latency_ms"));
    }

    #[test]
    fn constructs_skipped_result_with_reason() {
        let result = CheckResult::skipped(Layer::Dns, "DNS", "Gateway is down");

        assert_eq!(result.status, Status::Skipped);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].code, FindingCode::SkippedDueToLowerLayer);
    }

    #[test]
    fn maps_verdict_status_to_process_exit_code() {
        let ok = Report::new(vec![], Verdict::new(Status::Ok, "Healthy"));
        assert_eq!(ok.exit_code(), 0);

        let warn = Report::new(vec![], Verdict::new(Status::Warn, "Degraded"));
        assert_eq!(warn.exit_code(), 1);

        let fail = Report::new(vec![], Verdict::new(Status::Fail, "Down"));
        assert_eq!(fail.exit_code(), 2);
    }

    #[test]
    fn serializes_and_deserializes_check_result() {
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
    fn orders_layers_bottom_up() {
        let layers = Layer::all_ordered();
        assert_eq!(layers[0], Layer::Interface);
        assert_eq!(layers[4], Layer::Target);

        for window in layers.windows(2) {
            assert!(window[0] < window[1]);
        }
    }
}
