use std::{fmt, path::Path, time::Duration};

use hickory_client::proto::{dnssec::rdata::tsig::TsigAlgorithm, rr::domain::Name};
use serde::Deserialize;
use serde_with::{base64::Base64, serde_as, DisplayFromStr};
use thiserror::Error;

const DEFAULT_TTL: u32 = 300;
const DEFAULT_TIMEOUT: u64 = 5;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Ttl(pub u32);

impl Default for Ttl {
    fn default() -> Self {
        Self(DEFAULT_TTL)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Timeout(pub u64);

impl Timeout {
    pub fn to_duration(self) -> Duration {
        Duration::from_secs(self.0)
    }
}

impl Default for Timeout {
    fn default() -> Self {
        Self(DEFAULT_TIMEOUT)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn default_port(self) -> u16 {
        53
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Self::Udp
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Debug => f.write_str("debug"),
            Self::Trace => f.write_str("trace"),
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Global {
    pub name_server: String,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub zone: Option<Name>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub hostname: Option<Name>,
    #[serde(default)]
    pub ttl: Ttl,
    #[serde(default)]
    pub timeout: Timeout,
    #[serde(default)]
    pub protocol: Protocol,
    pub interface: Option<String>,
    #[serde(default)]
    pub log_level: LogLevel,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    HmacSha256,
    HmacSha384,
    HmacSha512,
}

impl From<Algorithm> for TsigAlgorithm {
    fn from(algo: Algorithm) -> Self {
        match algo {
            Algorithm::HmacSha256 => Self::HmacSha256,
            Algorithm::HmacSha384 => Self::HmacSha384,
            Algorithm::HmacSha512 => Self::HmacSha512,
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tsig {
    #[allow(dead_code)]
    pub name: String,
    pub algorithm: Algorithm,
    #[serde_as(as = "Base64")]
    pub secret: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub global: Global,
    pub tsig: Tsig,
}

pub fn load_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}
