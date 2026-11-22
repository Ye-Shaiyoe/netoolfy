use super::{GatewayDetails, InterfaceDetails, PingStats, Platform, PlatformError};
use async_trait::async_trait;
use std::fs;
use std::net::IpAddr;
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};

#[derive(Default)]
pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }

    fn read_wifi_signal_dbm(interface_name: &str) -> Option<i32> {
        let content = fs::read_to_string("/proc/net/wireless").ok()?;
        for line in content.lines().skip(2) {
            let line = line.trim();
            if let Some((iface, rest)) = line.split_once(':') {
                if iface.trim() == interface_name {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let level_str = parts[2].trim_end_matches('.');
                        if let Ok(dbm) = level_str.parse::<i32>() {
                            return Some(dbm);
                        }
                    }
                }
            }
        }
        None
    }
}

#[async_trait]
impl Platform for LinuxPlatform {
    async fn default_interface(&self) -> Result<Option<InterfaceDetails>, PlatformError> {
        let iface = match netdev::get_default_interface() {
            Ok(i) => Some(i),
            Err(_) => {
                let all = netdev::get_interfaces();
                all.into_iter().find(|i| {
                    i.is_up() && !i.is_loopback() && (!i.ipv4.is_empty() || !i.ipv6.is_empty())
                })
            }
        };

        let Some(iface) = iface else {
            return Ok(None);
        };

        let wifi_signal = Self::read_wifi_signal_dbm(&iface.name);
        let gateway_ip = iface
            .gateway
            .as_ref()
            .and_then(|gw| gw.ipv4.first().map(|ip| IpAddr::V4(*ip)))
            .or_else(|| {
                iface
                    .gateway
                    .as_ref()
                    .and_then(|gw| gw.ipv6.first().map(|ip| IpAddr::V6(*ip)))
            });

        Ok(Some(InterfaceDetails {
            name: iface.name.clone(),
            is_up: iface.is_up(),
            ipv4: iface.ipv4.iter().map(|net| net.addr()).collect(),
            ipv6: iface.ipv6.iter().map(|net| net.addr()).collect(),
            mac_addr: iface.mac_addr.map(|m| m.to_string()),
            wifi_signal_dbm: wifi_signal,
            gateway_ip,
        }))
    }

    async fn default_gateway(&self) -> Result<Option<GatewayDetails>, PlatformError> {
        if let Ok(gw) = netdev::get_default_gateway() {
            let ip = gw
                .ipv4
                .first()
                .map(|ip| IpAddr::V4(*ip))
                .or_else(|| gw.ipv6.first().map(|ip| IpAddr::V6(*ip)));

            if let Some(ip) = ip {
                let mac_str = gw.mac_addr.to_string();
                let mac_addr = if mac_str != "00:00:00:00:00:00" {
                    Some(mac_str)
                } else {
                    None
                };

                let iface_name = netdev::get_default_interface()
                    .map(|i| i.name)
                    .unwrap_or_else(|_| "default".into());

                return Ok(Some(GatewayDetails {
                    ip,
                    mac_addr,
                    interface_name: iface_name,
                }));
            }
        }

        if let Ok(Some(iface)) = self.default_interface().await {
            if let Some(ip) = iface.gateway_ip {
                return Ok(Some(GatewayDetails {
                    ip,
                    mac_addr: None,
                    interface_name: iface.name,
                }));
            }
        }

        Ok(None)
    }

    async fn ping(
        &self,
        target: IpAddr,
        count: u32,
        timeout: Duration,
    ) -> Result<PingStats, PlatformError> {
        let config = match target {
            IpAddr::V4(_) => Config::default(),
            IpAddr::V6(_) => Config::builder().kind(surge_ping::ICMP::V6).build(),
        };

        let client = Client::new(&config).map_err(|err| {
            let msg = err.to_string();
            if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                PlatformError::PingPermissionDenied(msg)
            } else {
                PlatformError::PingFailed(msg)
            }
        })?;

        let mut received = 0;
        let mut total_duration = Duration::ZERO;
        let payload = [0u8; 24];

        for seq in 0..count {
            let ident = (std::process::id() & 0xffff) as u16;
            let mut pinger = client.pinger(target, PingIdentifier(ident)).await;
            pinger.timeout(timeout);

            if let Ok((_, dur)) = pinger.ping(PingSequence(seq as u16), &payload).await {
                received += 1;
                total_duration += dur;
            }

            if seq + 1 < count {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        let loss_pct = if count > 0 {
            ((count - received) as f64 / count as f64) * 100.0
        } else {
            0.0
        };

        let avg_rtt_ms = if received > 0 {
            total_duration.as_secs_f64() * 1000.0 / received as f64
        } else {
            0.0
        };

        Ok(PingStats {
            packets_sent: count,
            packets_received: received,
            loss_pct,
            avg_rtt_ms,
        })
    }
}
