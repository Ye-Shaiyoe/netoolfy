//! Check framework: the [`Check`] trait, execution [`Context`], and
//! [`Runner`] that orchestrates checks layer by layer.
//!
//! The runner executes checks in layer order (interface → gateway → DNS →
//! internet → target). If any check at a layer returns [`Status::Fail`],
//! all checks in higher layers are automatically skipped — a failure at a
//! lower layer explains failures above it, so we avoid flooding the user
//! with redundant findings.

use crate::config::Config;
use crate::diagnosis;
use crate::model::{CheckResult, Layer, Report, Status};
use std::future::Future;
use std::pin::Pin;

// ── Check trait ───────────────────────────────────────────────────────

/// A single diagnostic check that probes one aspect of the network.
///
/// Checks must be safe to run concurrently within the same layer.
/// All I/O should go through the [`Context`] so tests can inject mocks.
pub trait Check: Send + Sync {
    /// Human-readable name of this check (e.g. "DNS", "Gateway").
    fn name(&self) -> &'static str;

    /// Which layer this check belongs to.
    fn layer(&self) -> Layer;

    /// Run the check and return a result.
    fn run<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>>;
}

// ── Context ───────────────────────────────────────────────────────────

/// Execution context passed to every check.
///
/// Holds configuration and (in future phases) injected trait objects for
/// DNS resolvers, ICMP pingers, etc., so tests can substitute mocks.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Runtime configuration.
    pub config: Config,
}

impl Context {
    /// Create a new context with the given configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

// ── Runner ────────────────────────────────────────────────────────────

/// Orchestrates checks layer by layer, skipping upper layers when a lower
/// layer has failed.
pub struct Runner {
    checks: Vec<Box<dyn Check>>,
}

impl Runner {
    /// Create a new runner with no checks registered.
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a check.
    pub fn add_check(&mut self, check: Box<dyn Check>) {
        self.checks.push(check);
    }

    /// Execute all registered checks in layer order.
    ///
    /// If any check at layer N returns [`Status::Fail`], all checks at
    /// layers above N are skipped with a [`FindingCode::SkippedDueToLowerLayer`]
    /// finding.
    pub async fn run(&self, ctx: &Context) -> Report {
        let mut results: Vec<CheckResult> = Vec::new();
        let mut failed_layer: Option<Layer> = None;

        for layer in Layer::all_ordered() {
            let layer_checks: Vec<&dyn Check> = self
                .checks
                .iter()
                .filter(|c| c.layer() == *layer)
                .map(|c| c.as_ref())
                .collect();

            if layer_checks.is_empty() {
                continue;
            }

            // If a lower layer already failed, skip this entire layer.
            if let Some(ref fl) = failed_layer {
                for check in &layer_checks {
                    results.push(CheckResult::skipped(
                        *layer,
                        check.name(),
                        &format!("{} check failed — skipping {}", fl, layer),
                    ));
                }
                continue;
            }

            // Run all checks in this layer.
            for check in &layer_checks {
                let result = check.run(ctx).await;
                if result.status == Status::Fail && failed_layer.is_none() {
                    failed_layer = Some(*layer);
                }
                results.push(result);
            }
        }

        let verdict = diagnosis::diagnose(&results);
        Report::new(results, verdict)
    }
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Finding, FindingCode};

    // ── Mock checks for testing ──────────────────────────────────────

    struct MockCheck {
        check_name: &'static str,
        check_layer: Layer,
        result_status: Status,
    }

    impl MockCheck {
        fn new(name: &'static str, layer: Layer, status: Status) -> Self {
            Self {
                check_name: name,
                check_layer: layer,
                result_status: status,
            }
        }
    }

    impl Check for MockCheck {
        fn name(&self) -> &'static str {
            self.check_name
        }

        fn layer(&self) -> Layer {
            self.check_layer
        }

        fn run<'a>(
            &'a self,
            _ctx: &'a Context,
        ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>> {
            let status = self.result_status;
            let layer = self.check_layer;
            let name = self.check_name;
            Box::pin(async move {
                let finding = match status {
                    Status::Ok => Finding::new(FindingCode::AllGood, format!("{name} is healthy")),
                    Status::Warn => Finding::new(
                        FindingCode::GatewayLatencyHigh,
                        format!("{name} has warnings"),
                    ),
                    Status::Fail => Finding::new(FindingCode::NoGateway, format!("{name} failed")),
                    Status::Skipped => Finding::new(
                        FindingCode::SkippedDueToLowerLayer,
                        format!("{name} skipped"),
                    ),
                };
                CheckResult::new(layer, name, status).with_finding(finding)
            })
        }
    }

    // ── Tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn runner_all_ok() {
        let mut runner = Runner::new();
        runner.add_check(Box::new(MockCheck::new(
            "Interface",
            Layer::Interface,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new(
            "Gateway",
            Layer::Gateway,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new("DNS", Layer::Dns, Status::Ok)));

        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        assert_eq!(report.results.len(), 3);
        for r in &report.results {
            assert_eq!(r.status, Status::Ok);
        }
        assert_eq!(report.verdict.overall_status, Status::Ok);
    }

    #[tokio::test]
    async fn runner_lower_layer_fail_skips_upper() {
        let mut runner = Runner::new();
        runner.add_check(Box::new(MockCheck::new(
            "Interface",
            Layer::Interface,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new(
            "Gateway",
            Layer::Gateway,
            Status::Fail,
        )));
        runner.add_check(Box::new(MockCheck::new("DNS", Layer::Dns, Status::Ok)));
        runner.add_check(Box::new(MockCheck::new(
            "Internet",
            Layer::Internet,
            Status::Ok,
        )));

        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        assert_eq!(report.results.len(), 4);

        // Interface should be Ok.
        assert_eq!(report.results[0].status, Status::Ok);
        // Gateway should be Fail.
        assert_eq!(report.results[1].status, Status::Fail);
        // DNS and Internet should be Skipped.
        assert_eq!(report.results[2].status, Status::Skipped);
        assert_eq!(report.results[3].status, Status::Skipped);

        assert_eq!(report.verdict.overall_status, Status::Fail);
    }

    #[tokio::test]
    async fn runner_warn_does_not_skip_upper() {
        let mut runner = Runner::new();
        runner.add_check(Box::new(MockCheck::new(
            "Interface",
            Layer::Interface,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new(
            "Gateway",
            Layer::Gateway,
            Status::Warn,
        )));
        runner.add_check(Box::new(MockCheck::new("DNS", Layer::Dns, Status::Ok)));

        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        // Warning should NOT skip upper layers.
        assert_eq!(report.results[0].status, Status::Ok);
        assert_eq!(report.results[1].status, Status::Warn);
        assert_eq!(report.results[2].status, Status::Ok);
    }

    #[tokio::test]
    async fn runner_empty_produces_report() {
        let runner = Runner::new();
        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        assert!(report.results.is_empty());
        assert_eq!(report.verdict.overall_status, Status::Ok);
    }

    #[tokio::test]
    async fn runner_interface_fail_skips_all_upper() {
        let mut runner = Runner::new();
        runner.add_check(Box::new(MockCheck::new(
            "Interface",
            Layer::Interface,
            Status::Fail,
        )));
        runner.add_check(Box::new(MockCheck::new(
            "Gateway",
            Layer::Gateway,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new("DNS", Layer::Dns, Status::Ok)));
        runner.add_check(Box::new(MockCheck::new(
            "Internet",
            Layer::Internet,
            Status::Ok,
        )));
        runner.add_check(Box::new(MockCheck::new(
            "Target",
            Layer::Target,
            Status::Ok,
        )));

        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        assert_eq!(report.results[0].status, Status::Fail);
        // All 4 upper layers should be skipped.
        for result in &report.results[1..] {
            assert_eq!(
                result.status,
                Status::Skipped,
                "Expected {} to be skipped",
                result.name
            );
        }
    }
}
