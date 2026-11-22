use super::{Check, Context};
use crate::config::{GATEWAY_FAIL_MS, GATEWAY_WARN_MS, LOSS_FAIL_PCT, LOSS_WARN_PCT};
use crate::model::{CheckResult, Finding, FindingCode, Layer, MetricValue, Status};
use crate::platform::PlatformError;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Default)]
pub struct GatewayCheck;

impl GatewayCheck {
    pub fn new() -> Self {
        Self
    }

    async fn probe_tcp_reachability(ip: std::net::IpAddr) -> bool {
        let ports = [80, 53, 443];
        for port in ports {
            let addr = SocketAddr::new(ip, port);
            if timeout(Duration::from_millis(500), TcpStream::connect(addr))
                .await
                .is_ok()
            {
                return true;
            }
        }
        false
    }
}

impl Check for GatewayCheck {
    fn name(&self) -> &'static str {
        "Gateway"
    }

    fn layer(&self) -> Layer {
        Layer::Gateway
    }

    fn run<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>> {
        Box::pin(async move {
            let gw = match ctx.platform.default_gateway().await {
                Err(PlatformError::Unsupported(reason)) => {
                    return CheckResult::skipped(Layer::Gateway, self.name(), &reason);
                }
                Err(err) => {
                    return CheckResult::new(Layer::Gateway, self.name(), Status::Fail)
                        .with_finding(
                            Finding::new(
                                FindingCode::NoGateway,
                                format!("Failed to detect default gateway: {err}"),
                            )
                            .with_fix("Verify local network configuration or router connection"),
                        );
                }
                Ok(None) => {
                    return CheckResult::new(Layer::Gateway, self.name(), Status::Fail)
                        .with_finding(
                            Finding::new(FindingCode::NoGateway, "No default gateway found")
                                .with_fix("Check router connection or local DHCP settings"),
                        );
                }
                Ok(Some(gw)) => gw,
            };

            match ctx
                .platform
                .ping(gw.ip, ctx.config.ping_count, ctx.config.ping_timeout)
                .await
            {
                Ok(stats) => {
                    if stats.packets_received == 0 {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Fail).with_finding(
                            Finding::new(
                                FindingCode::GatewayUnreachable,
                                format!("Gateway {} unreachable (100% loss)", gw.ip),
                            )
                            .with_fix("Restart your router or check network cabling")
                            .with_metric("packet_loss_pct", MetricValue::Float(100.0)),
                        )
                    } else if stats.loss_pct > LOSS_FAIL_PCT {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Fail).with_finding(
                            Finding::new(
                                FindingCode::PacketLoss,
                                format!(
                                    "Gateway {} packet loss high ({:.1}%)",
                                    gw.ip, stats.loss_pct
                                ),
                            )
                            .with_fix("Check Wi-Fi interference or damaged network cable")
                            .with_metric("packet_loss_pct", MetricValue::Float(stats.loss_pct)),
                        )
                    } else if stats.avg_rtt_ms > GATEWAY_FAIL_MS as f64 {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Fail).with_finding(
                            Finding::new(
                                FindingCode::GatewayLatencyHigh,
                                format!(
                                    "Gateway {} latency critical ({:.1} ms)",
                                    gw.ip, stats.avg_rtt_ms
                                ),
                            )
                            .with_fix("Restart router or check for heavy local network congestion")
                            .with_metric("latency_ms", MetricValue::Float(stats.avg_rtt_ms)),
                        )
                    } else if stats.loss_pct > LOSS_WARN_PCT {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Warn).with_finding(
                            Finding::new(
                                FindingCode::PacketLoss,
                                format!(
                                    "Gateway {} has packet loss ({:.1}%)",
                                    gw.ip, stats.loss_pct
                                ),
                            )
                            .with_metric("packet_loss_pct", MetricValue::Float(stats.loss_pct)),
                        )
                    } else if stats.avg_rtt_ms > GATEWAY_WARN_MS as f64 {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Warn).with_finding(
                            Finding::new(
                                FindingCode::GatewayLatencyHigh,
                                format!(
                                    "Gateway {} latency is elevated ({:.1} ms)",
                                    gw.ip, stats.avg_rtt_ms
                                ),
                            )
                            .with_metric("latency_ms", MetricValue::Float(stats.avg_rtt_ms)),
                        )
                    } else {
                        CheckResult::new(Layer::Gateway, self.name(), Status::Ok).with_finding(
                            Finding::new(
                                FindingCode::AllGood,
                                format!("{} reachable, {:.1} ms", gw.ip, stats.avg_rtt_ms),
                            )
                            .with_metric("latency_ms", MetricValue::Float(stats.avg_rtt_ms)),
                        )
                    }
                }
                Err(PlatformError::PingPermissionDenied(_)) => {
                    let tcp_reachable = Self::probe_tcp_reachability(gw.ip).await;
                    let msg = if tcp_reachable {
                        format!("{} reachable via TCP (ICMP ping restricted by OS)", gw.ip)
                    } else {
                        format!("{} detected on link (ICMP ping restricted by OS)", gw.ip)
                    };

                    CheckResult::new(Layer::Gateway, self.name(), Status::Ok)
                        .with_finding(Finding::new(FindingCode::AllGood, msg))
                }
                Err(err) => CheckResult::new(Layer::Gateway, self.name(), Status::Fail)
                    .with_finding(
                        Finding::new(
                            FindingCode::GatewayUnreachable,
                            format!("Failed to probe gateway {}: {err}", gw.ip),
                        )
                        .with_fix("Check router power and physical connection"),
                    ),
            }
        })
    }
}
