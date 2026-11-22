use super::{Check, Context};
use crate::config::{WIFI_SIGNAL_FAIL_DBM, WIFI_SIGNAL_WARN_DBM};
use crate::model::{CheckResult, Finding, FindingCode, Layer, MetricValue, Status};
use crate::platform::PlatformError;
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
pub struct InterfaceCheck;

impl InterfaceCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Check for InterfaceCheck {
    fn name(&self) -> &'static str {
        "Interface"
    }

    fn layer(&self) -> Layer {
        Layer::Interface
    }

    fn run<'a>(
        &'a self,
        ctx: &'a Context,
    ) -> Pin<Box<dyn Future<Output = CheckResult> + Send + 'a>> {
        Box::pin(async move {
            match ctx.platform.default_interface().await {
                Err(PlatformError::Unsupported(reason)) => {
                    CheckResult::skipped(Layer::Interface, self.name(), &reason)
                }
                Err(err) => CheckResult::new(Layer::Interface, self.name(), Status::Fail)
                    .with_finding(
                        Finding::new(
                            FindingCode::NoInterface,
                            format!("Failed to inspect network interfaces: {err}"),
                        )
                        .with_fix("Verify your network hardware and drivers"),
                    ),
                Ok(None) => CheckResult::new(Layer::Interface, self.name(), Status::Fail)
                    .with_finding(
                        Finding::new(
                            FindingCode::NoInterface,
                            "No active network interface found",
                        )
                        .with_fix("Check your network cable or Wi-Fi connection"),
                    ),
                Ok(Some(iface)) => {
                    if !iface.is_up {
                        return CheckResult::new(Layer::Interface, self.name(), Status::Fail)
                            .with_finding(
                                Finding::new(
                                    FindingCode::NoInterface,
                                    format!("Interface {} is down", iface.name),
                                )
                                .with_fix("Enable your network interface in system settings"),
                            );
                    }

                    let primary_ip = iface
                        .ipv4
                        .first()
                        .map(|ip| ip.to_string())
                        .or_else(|| iface.ipv6.first().map(|ip| ip.to_string()));

                    let Some(ip_str) = primary_ip else {
                        return CheckResult::new(Layer::Interface, self.name(), Status::Fail)
                            .with_finding(
                                Finding::new(
                                    FindingCode::NoIpAddress,
                                    format!("Interface {} has no assigned IP address", iface.name),
                                )
                                .with_fix("Check DHCP settings or renew your IP address lease"),
                            );
                    };

                    if let Some(dbm) = iface.wifi_signal_dbm {
                        if dbm <= WIFI_SIGNAL_FAIL_DBM {
                            return CheckResult::new(Layer::Interface, self.name(), Status::Fail)
                                .with_finding(
                                    Finding::new(
                                        FindingCode::WeakWifiSignal,
                                        format!(
                                        "Interface {} Wi-Fi signal is critically weak ({dbm} dBm)",
                                        iface.name
                                    ),
                                    )
                                    .with_fix("Move closer to your Wi-Fi router")
                                    .with_metric("wifi_signal_dbm", MetricValue::Int(dbm as i64)),
                                );
                        }

                        if dbm <= WIFI_SIGNAL_WARN_DBM {
                            return CheckResult::new(Layer::Interface, self.name(), Status::Warn)
                                .with_finding(
                                    Finding::new(
                                        FindingCode::WeakWifiSignal,
                                        format!(
                                            "Interface {} Wi-Fi signal is weak ({dbm} dBm)",
                                            iface.name
                                        ),
                                    )
                                    .with_fix("Move closer to your Wi-Fi router")
                                    .with_metric("wifi_signal_dbm", MetricValue::Int(dbm as i64)),
                                );
                        }

                        return CheckResult::new(Layer::Interface, self.name(), Status::Ok)
                            .with_finding(
                                Finding::new(
                                    FindingCode::AllGood,
                                    format!(
                                        "{} up, IP {}, signal good ({dbm} dBm)",
                                        iface.name, ip_str
                                    ),
                                )
                                .with_metric("wifi_signal_dbm", MetricValue::Int(dbm as i64)),
                            );
                    }

                    CheckResult::new(Layer::Interface, self.name(), Status::Ok).with_finding(
                        Finding::new(
                            FindingCode::AllGood,
                            format!("{} up, IP {}", iface.name, ip_str),
                        ),
                    )
                }
            }
        })
    }
}
