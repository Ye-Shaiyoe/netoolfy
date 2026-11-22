pub mod gateway;
pub mod interface;

use crate::config::Config;
use crate::diagnosis;
use crate::model::{CheckResult, Layer, Report, Status};
use crate::platform::{self, Platform};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait Check: Send + Sync {
    fn name(&self) -> &'static str;
    fn layer(&self) -> Layer;
    fn run<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>>;
}

#[derive(Clone)]
pub struct Context {
    pub config: Config,
    pub platform: Arc<dyn Platform>,
}

impl Context {
    pub fn new(config: Config, platform: Arc<dyn Platform>) -> Self {
        Self { config, platform }
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            platform: platform::current_platform(),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            config: Config::default(),
            platform: platform::current_platform(),
        }
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
    use crate::checks::gateway::GatewayCheck;
    use crate::checks::interface::InterfaceCheck;
    use crate::model::FindingCode;
    use crate::platform::{GatewayDetails, InterfaceDetails, PingStats, PlatformError};
    use async_trait::async_trait;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    struct MockPlatform {
        interface_res: Result<Option<InterfaceDetails>, PlatformError>,
        gateway_res: Result<Option<GatewayDetails>, PlatformError>,
        ping_res: Result<PingStats, PlatformError>,
    }

    impl Default for MockPlatform {
        fn default() -> Self {
            Self {
                interface_res: Ok(Some(InterfaceDetails {
                    name: "wlan0".into(),
                    is_up: true,
                    ipv4: vec![Ipv4Addr::new(192, 168, 1, 23)],
                    ipv6: vec![],
                    mac_addr: Some("aa:bb:cc:dd:ee:ff".into()),
                    wifi_signal_dbm: Some(-55),
                    gateway_ip: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
                })),
                gateway_res: Ok(Some(GatewayDetails {
                    ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                    mac_addr: Some("00:11:22:33:44:55".into()),
                    interface_name: "wlan0".into(),
                })),
                ping_res: Ok(PingStats {
                    packets_sent: 5,
                    packets_received: 5,
                    loss_pct: 0.0,
                    avg_rtt_ms: 2.1,
                }),
            }
        }
    }

    #[async_trait]
    impl Platform for MockPlatform {
        async fn default_interface(&self) -> Result<Option<InterfaceDetails>, PlatformError> {
            match &self.interface_res {
                Ok(Some(i)) => Ok(Some(i.clone())),
                Ok(None) => Ok(None),
                Err(e) => Err(PlatformError::InterfaceQuery(e.to_string())),
            }
        }

        async fn default_gateway(&self) -> Result<Option<GatewayDetails>, PlatformError> {
            match &self.gateway_res {
                Ok(Some(g)) => Ok(Some(g.clone())),
                Ok(None) => Ok(None),
                Err(e) => Err(PlatformError::GatewayQuery(e.to_string())),
            }
        }

        async fn ping(
            &self,
            _target: IpAddr,
            _count: u32,
            _timeout: Duration,
        ) -> Result<PingStats, PlatformError> {
            match &self.ping_res {
                Ok(p) => Ok(p.clone()),
                Err(PlatformError::PingPermissionDenied(msg)) => {
                    Err(PlatformError::PingPermissionDenied(msg.clone()))
                }
                Err(e) => Err(PlatformError::PingFailed(e.to_string())),
            }
        }
    }

    #[tokio::test]
    async fn interface_reports_ok_for_healthy_interface() {
        let platform = Arc::new(MockPlatform::default());
        let ctx = Context::new(Config::default(), platform);
        let check = InterfaceCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Ok);
        assert!(result.findings[0].message.contains("wlan0 up"));
        assert!(result.findings[0].message.contains("192.168.1.23"));
    }

    #[tokio::test]
    async fn interface_fails_when_no_interface_found() {
        let mock = MockPlatform {
            interface_res: Ok(None),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = InterfaceCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Fail);
        assert_eq!(result.findings[0].code, FindingCode::NoInterface);
    }

    #[tokio::test]
    async fn interface_fails_when_interface_down() {
        let mut mock = MockPlatform::default();
        if let Ok(Some(ref mut iface)) = mock.interface_res {
            iface.is_up = false;
        }
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = InterfaceCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Fail);
        assert_eq!(result.findings[0].code, FindingCode::NoInterface);
        assert!(result.findings[0].message.contains("is down"));
    }

    #[tokio::test]
    async fn interface_fails_when_no_ip_assigned() {
        let mut mock = MockPlatform::default();
        if let Ok(Some(ref mut iface)) = mock.interface_res {
            iface.ipv4.clear();
            iface.ipv6.clear();
        }
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = InterfaceCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Fail);
        assert_eq!(result.findings[0].code, FindingCode::NoIpAddress);
    }

    #[tokio::test]
    async fn interface_warns_on_weak_wifi_signal() {
        let mut mock = MockPlatform::default();
        if let Ok(Some(ref mut iface)) = mock.interface_res {
            iface.wifi_signal_dbm = Some(-78);
        }
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = InterfaceCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Warn);
        assert_eq!(result.findings[0].code, FindingCode::WeakWifiSignal);
    }

    #[tokio::test]
    async fn gateway_reports_ok_when_ping_healthy() {
        let platform = Arc::new(MockPlatform::default());
        let ctx = Context::new(Config::default(), platform);
        let check = GatewayCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Ok);
        assert!(result.findings[0].message.contains("192.168.1.1 reachable"));
    }

    #[tokio::test]
    async fn gateway_fails_when_no_gateway_found() {
        let mock = MockPlatform {
            gateway_res: Ok(None),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = GatewayCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Fail);
        assert_eq!(result.findings[0].code, FindingCode::NoGateway);
    }

    #[tokio::test]
    async fn gateway_fails_when_100_percent_loss() {
        let mock = MockPlatform {
            ping_res: Ok(PingStats {
                packets_sent: 5,
                packets_received: 0,
                loss_pct: 100.0,
                avg_rtt_ms: 0.0,
            }),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = GatewayCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Fail);
        assert_eq!(result.findings[0].code, FindingCode::GatewayUnreachable);
    }

    #[tokio::test]
    async fn gateway_warns_on_high_latency() {
        let mock = MockPlatform {
            ping_res: Ok(PingStats {
                packets_sent: 5,
                packets_received: 5,
                loss_pct: 0.0,
                avg_rtt_ms: 45.0,
            }),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = GatewayCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Warn);
        assert_eq!(result.findings[0].code, FindingCode::GatewayLatencyHigh);
    }

    #[tokio::test]
    async fn gateway_handles_ping_permission_denied() {
        let mock = MockPlatform {
            ping_res: Err(PlatformError::PingPermissionDenied(
                "Operation not permitted".into(),
            )),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));
        let check = GatewayCheck::new();

        let result = check.run(&ctx).await;

        assert_eq!(result.status, Status::Ok);
        assert!(result.findings[0].message.contains("restricted by OS"));
    }

    #[tokio::test]
    async fn skips_gateway_when_interface_fails() {
        let mock = MockPlatform {
            interface_res: Ok(None),
            ..Default::default()
        };
        let ctx = Context::new(Config::default(), Arc::new(mock));

        let mut runner = Runner::new();
        runner.add_check(Box::new(InterfaceCheck::new()));
        runner.add_check(Box::new(GatewayCheck::new()));

        let report = runner.run(&ctx).await;

        assert_eq!(report.results[0].status, Status::Fail);
        assert_eq!(report.results[1].status, Status::Skipped);
        assert_eq!(report.verdict.overall_status, Status::Fail);
    }
}
