use crate::cli::Cli;
use std::time::Duration;

pub const PING_TIMEOUT: Duration = Duration::from_secs(3);
pub const DNS_TIMEOUT: Duration = Duration::from_secs(5);
pub const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const TLS_TIMEOUT: Duration = Duration::from_secs(5);

pub const DNS_SLOW_MS: u64 = 100;
pub const DNS_FAIL_MS: u64 = 2000;
pub const GATEWAY_WARN_MS: u64 = 20;
pub const GATEWAY_FAIL_MS: u64 = 200;
pub const LOSS_WARN_PCT: f64 = 2.0;
pub const LOSS_FAIL_PCT: f64 = 10.0;
pub const JITTER_WARN_MS: f64 = 10.0;
pub const JITTER_FAIL_MS: f64 = 50.0;
pub const PING_COUNT: u32 = 5;
pub const CERT_EXPIRY_WARN_DAYS: u32 = 14;

pub const PUBLIC_DNS_SERVERS: &[&str] = &["1.1.1.1", "8.8.8.8", "9.9.9.9"];
pub const DNS_TEST_DOMAINS: &[&str] = &["google.com", "cloudflare.com", "example.com"];
pub const INTERNET_TARGETS: &[&str] = &["1.1.1.1", "8.8.8.8"];

#[derive(Debug, Clone)]
pub struct Config {
    pub json: bool,
    pub no_color: bool,
    pub ping_count: u32,
    pub ping_timeout: Duration,
    pub dns_timeout: Duration,
}

impl Config {
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
