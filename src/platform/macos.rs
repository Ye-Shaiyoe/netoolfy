use super::{GatewayDetails, InterfaceDetails, PingStats, Platform, PlatformError};
use async_trait::async_trait;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Default)]
pub struct MacOsPlatform;

impl MacOsPlatform {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Platform for MacOsPlatform {
    async fn default_interface(&self) -> Result<Option<InterfaceDetails>, PlatformError> {
        Err(PlatformError::Unsupported(
            "macOS interface check is planned for Phase 7".into(),
        ))
    }

    async fn default_gateway(&self) -> Result<Option<GatewayDetails>, PlatformError> {
        Err(PlatformError::Unsupported(
            "macOS gateway check is planned for Phase 7".into(),
        ))
    }

    async fn ping(
        &self,
        _target: IpAddr,
        _count: u32,
        _timeout: Duration,
    ) -> Result<PingStats, PlatformError> {
        Err(PlatformError::Unsupported(
            "macOS ping check is planned for Phase 7".into(),
        ))
    }
}
