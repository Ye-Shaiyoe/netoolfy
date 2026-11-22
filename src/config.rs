//! Configuration defaults: targets, timeouts, and thresholds.
//!
//! All magic numbers live here as named constants so they are easy to find
//! and override.

use crate::cli::Cli;
use std::time::Duration;

// ── Timeouts ──────────────────────────────────────────────────────────

/// Default timeout for a single ICMP ping probe.
pub const PING_TIMEOUT: Duration = Duration::from_secs(3);

/// Default timeout for a DNS lookup.
pub const DNS_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for a TCP port check.
pub const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for a TLS handshake.
pub const TLS_TIMEOUT: Duration = Duration::from_secs(5);

// ── Thresholds ────────────────────────────────────────────────────────

/// DNS latency above this is considered slow (milliseconds).
pub const DNS_SLOW_MS: u64 = 100;

/// DNS latency above this is considered failing (milliseconds).
pub const DNS_FAIL_MS: u64 = 2000;

/// Gateway ping latency above this is a warning (milliseconds).
pub const GATEWAY_WARN_MS: u64 = 20;

/// Gateway ping latency above this is a failure (milliseconds).
pub const GATEWAY_FAIL_MS: u64 = 200;

/// Packet loss above this percentage is a warning.
pub const LOSS_WARN_PCT: f64 = 2.0;

/// Packet loss above this percentage is a failure.
pub const LOSS_FAIL_PCT: f64 = 10.0;

/// Jitter above this is a warning (milliseconds).
pub const JITTER_WARN_MS: f64 = 10.0;

/// Jitter above this is a failure (milliseconds).
pub const JITTER_FAIL_MS: f64 = 50.0;

/// Number of ICMP echo requests to send in the internet check.
pub const PING_COUNT: u32 = 5;

/// TLS certificate expiry warning threshold (days).
pub const CERT_EXPIRY_WARN_DAYS: u32 = 14;

// ── Default targets ───────────────────────────────────────────────────

/// Well-known public DNS resolvers used for comparison.
pub const PUBLIC_DNS_SERVERS: &[&str] = &["1.1.1.1", "8.8.8.8", "9.9.9.9"];

/// Default domains to resolve during the DNS check.
pub const DNS_TEST_DOMAINS: &[&str] = &["google.com", "cloudflare.com", "example.com"];

/// Default ICMP targets for the internet connectivity check.
pub const INTERNET_TARGETS: &[&str] = &["1.1.1.1", "8.8.8.8"];

// ── Runtime config ────────────────────────────────────────────────────

/// Runtime configuration built from CLI flags and defaults.
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether to output JSON.
    pub json: bool,

    /// Whether to disable colors.
    pub no_color: bool,

    /// Number of pings to send.
    pub ping_count: u32,

    /// Ping timeout.
    pub ping_timeout: Duration,

    /// DNS timeout.
    pub dns_timeout: Duration,
}

impl Config {
    /// Build a [`Config`] from CLI arguments, filling in defaults.
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            json: cli.json,
            no_color: cli.no_color,
            ping_count: PING_COUNT,
            ping_timeout: PING_TIMEOUT,
            dns_timeout: DNS_TIMEOUT,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            json: false,
            no_color: false,
            ping_count: PING_COUNT,
            ping_timeout: PING_TIMEOUT,
            dns_timeout: DNS_TIMEOUT,
        }
    }
}
