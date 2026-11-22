use crate::config::Config;
use crate::diagnosis;
use crate::model::{CheckResult, Layer, Report, Status};
use std::future::Future;
use std::pin::Pin;

pub trait Check: Send + Sync {
    fn name(&self) -> &'static str;
    fn layer(&self) -> Layer;
    fn run<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>>;
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub config: Config,
}

impl Context {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub struct Runner {
    checks: Vec<Box<dyn Check>>,
}

impl Runner {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add_check(&mut self, check: Box<dyn Check>) {
        self.checks.push(check);
    }

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

            if let Some(ref fl) = failed_layer {
                for check in &layer_checks {
                    results.push(CheckResult::skipped(
                        *layer,
                        check.name(),
                        &format!("{fl} check failed — skipping {layer}"),
                    ));
                }
                continue;
            }

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

    struct MockCheck {
        name: &'static str,
        layer: Layer,
        status: Status,
    }

    impl MockCheck {
        fn new(name: &'static str, layer: Layer, status: Status) -> Self {
            Self {
                name,
                layer,
                status,
            }
        }
    }

    impl Check for MockCheck {
        fn name(&self) -> &'static str {
            self.name
        }

        fn layer(&self) -> Layer {
            self.layer
        }

        fn run<'a>(
            &'a self,
            _ctx: &'a Context,
        ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>> {
            let status = self.status;
            let layer = self.layer;
            let name = self.name;
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

    #[tokio::test]
    async fn reports_ok_when_all_checks_pass() {
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
        assert!(report.results.iter().all(|r| r.status == Status::Ok));
        assert_eq!(report.verdict.overall_status, Status::Ok);
    }

    #[tokio::test]
    async fn skips_upper_layers_when_lower_layer_fails() {
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
        assert_eq!(report.results[0].status, Status::Ok);
        assert_eq!(report.results[1].status, Status::Fail);
        assert_eq!(report.results[2].status, Status::Skipped);
        assert_eq!(report.results[3].status, Status::Skipped);
        assert_eq!(report.verdict.overall_status, Status::Fail);
    }

    #[tokio::test]
    async fn runs_upper_layers_when_lower_layer_has_warning() {
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

        assert_eq!(report.results[0].status, Status::Ok);
        assert_eq!(report.results[1].status, Status::Warn);
        assert_eq!(report.results[2].status, Status::Ok);
    }

    #[tokio::test]
    async fn returns_ok_verdict_when_no_checks_registered() {
        let runner = Runner::new();
        let ctx = Context::default();
        let report = runner.run(&ctx).await;

        assert!(report.results.is_empty());
        assert_eq!(report.verdict.overall_status, Status::Ok);
    }

    #[tokio::test]
    async fn skips_all_upper_layers_when_interface_fails() {
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
        for result in &report.results[1..] {
            assert_eq!(result.status, Status::Skipped);
        }
    }
}
