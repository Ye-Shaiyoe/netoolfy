pub mod linux;
pub mod macos;
pub mod windows;

use async_trait::async_trait;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("failed to query interface: {0}")]
    InterfaceQuery(String),
    #[error("failed to query default gateway: {0}")]
    GatewayQuery(String),
    #[error("ping failed: {0}")]
    PingFailed(String),
    #[error("missing socket privileges for ICMP: {0}")]
    PingPermissionDenied(String),
    #[error("unsupported platform: {0}")]
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceDetails {
    pub name: String,
    pub is_up: bool,
    pub ipv4: Vec<Ipv4Addr>,
    pub ipv6: Vec<Ipv6Addr>,
    pub mac_addr: Option<String>,
    pub wifi_signal_dbm: Option<i32>,
    pub gateway_ip: Option<IpAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayDetails {
    pub ip: IpAddr,
    pub mac_addr: Option<String>,
    pub interface_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PingStats {
    pub packets_sent: u32,
    pub packets_received: u32,
    pub loss_pct: f64,
    pub avg_rtt_ms: f64,
}

#[async_trait]
pub trait Platform: Send + Sync {
    async fn default_interface(&self) -> Result<Option<InterfaceDetails>, PlatformError>;
    async fn default_gateway(&self) -> Result<Option<GatewayDetails>, PlatformError>;
    async fn ping(
        &self,
        target: IpAddr,
        count: u32,
        timeout: Duration,
    ) -> Result<PingStats, PlatformError>;
}

pub fn current_platform() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxPlatform::new())
    }

    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOsPlatform::new())
    }

    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsPlatform::new())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Arc::new(linux::LinuxPlatform::new())
    }
}
