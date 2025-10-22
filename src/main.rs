mod config;
mod dns;
mod iface;
mod ip;

use std::{
    borrow::Cow,
    ffi::OsString,
    io::{self, IsTerminal},
    net::SocketAddr,
    path::PathBuf,
    process::ExitCode,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use clap::Parser;
use hickory_client::{
    ClientError,
    client::ClientHandle,
    proto::{
        ProtoError,
        dnssec::tsig::TSigner,
        op::{response_code::ResponseCode, update_message::UpdateMessage},
        rr::{DNSClass, Name, RecordType},
    },
};
use iface::Interfaces;
use tracing::{debug, error, info, trace};
use tracing_subscriber::{EnvFilter, filter::Directive};

// Same as nsupdate
const DEFAULT_FUDGE: u16 = 300;

static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Could not load config: {0:?}")]
    Config(PathBuf, #[source] config::Error),
    #[error("Could not resolve DNS host: {0}")]
    ResolveDnsHost(String, #[source] io::Error),
    #[error("Error when querying interfaces")]
    Interface(#[from] iface::Error),
    #[error("Interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("Hostname has invalid UTF-8: {0:?}")]
    InvalidHostnameUtf8(OsString),
    #[error("Hostname is not a valid DNS name: {0:?}")]
    InvalidHostnameDns(OsString, #[source] ProtoError),
    #[error("Error when creating DNS client for: {0}")]
    ClientCreate(SocketAddr, #[source] ProtoError),
    #[error("Error when querying DNS server: {0}")]
    ClientQuery(SocketAddr, #[source] ClientError),
    #[error("Authoritative zone not found: {0}")]
    AuthoritativeZoneNotFound(Name),
    #[error("Bad server response: {0}")]
    BadResponse(ResponseCode),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Parser)]
#[clap(about, author, version)]
struct Opts {
    /// Path to config file
    #[clap(short, long)]
    config: PathBuf,
}

async fn update_dns(server: SocketAddr, config: &config::Config) -> Result<()> {
    let ifaces = Interfaces::new()?;
    let iface = match &config.global.interface {
        Some(s) => s.as_str(),
        None => {
            debug!("Autodetecting interface from source IP of TCP connection to {server}");
            ifaces.get_iface_by_tcp_source_ip(server).await?
        }
    };
    debug!("Interface: {iface:?}");

    let addrs = ifaces
        .get_addrs_by_name(iface)
        .ok_or_else(|| Error::InterfaceNotFound(iface.to_string()))?;
    let valid_addrs = addrs
        .into_iter()
        .filter(|ip| ip::is_suitable_ip(*ip))
        .collect::<Vec<_>>();
    debug!("Addresses: {valid_addrs:?}");

    let name = match &config.global.hostname {
        Some(n) => Cow::Borrowed(n),
        None => {
            let hostname = gethostname::gethostname();
            let hostname_str = hostname
                .to_str()
                .ok_or_else(|| Error::InvalidHostnameUtf8(hostname.clone()))?;
            let name = Name::from_str(hostname_str)
                .map_err(|e| Error::InvalidHostnameDns(hostname.clone(), e))?;
            Cow::Owned(name)
        }
    };
    debug!("Hostname: {name}");

    let tsig = TSigner::new(
        config.tsig.secret.clone(),
        config.tsig.algorithm.into(),
        name.clone().into_owned(),
        DEFAULT_FUDGE,
    )
    .unwrap();

    let mut client = dns::new_client(
        server,
        config.global.protocol,
        config.global.timeout.to_duration(),
        tsig,
    )
    .await
    .map_err(|e| Error::ClientCreate(server, e))?;

    let zone = match &config.global.zone {
        Some(n) => Cow::Borrowed(n),
        None => {
            debug!("Querying SOA for: {name}");

            let response = client
                .query(name.clone().into_owned(), DNSClass::IN, RecordType::SOA)
                .await
                .map_err(|e| Error::ClientQuery(server, e))?;
            let authority = response.name_servers();
            if authority.is_empty() {
                return Err(Error::AuthoritativeZoneNotFound(name.into_owned()));
            }

            Cow::Owned(authority[0].name().clone())
        }
    };
    debug!("Zone: {zone}");

    let request = dns::replace_addrs_message(&zone, &name, config.global.ttl.0, &valid_addrs);
    trace!("Update request: {request:?}");

    for record in request.updates() {
        info!("Record update: {record}");
    }

    let response = dns::send_message(&client, request)
        .await
        .map_err(|e| Error::ClientQuery(server, e))?;
    trace!("Update response: {response:?}");

    let code = response.response_code();
    if code != ResponseCode::NoError {
        return Err(Error::BadResponse(code));
    }

    Ok(())
}

async fn main_wrapper() -> Result<()> {
    let opts: Opts = Opts::parse();
    let config =
        config::load_config(&opts.config).map_err(|e| Error::Config(opts.config.clone(), e))?;

    let default_directive: Directive = format!(
        "{}={}",
        env!("CARGO_PKG_NAME").replace('-', "_"),
        config.global.log_level,
    )
    .parse()
    .expect("Broken hardcoded directive");

    let filter = EnvFilter::builder()
        .with_default_directive(default_directive)
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal())
        .with_env_filter(filter)
        .without_time()
        .init();

    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);

    trace!("Loaded config: {config:?}");

    let server_with_port = match &config.global.name_server {
        h if h.contains(':') => Cow::Borrowed(h.as_str()),
        h => Cow::Owned(format!("{h}:{}", config.global.protocol.default_port())),
    };
    debug!("Name server: {server_with_port:?}");

    let servers = tokio::net::lookup_host(server_with_port.as_ref())
        .await
        .map_err(|e| Error::ResolveDnsHost(server_with_port.to_string(), e))?
        .collect::<Vec<_>>();
    debug!("Resolved name servers: {servers:?}");

    let mut last_error = None;

    for server in servers {
        debug!("Attempting to use name server: {server}");

        match update_dns(server, &config).await {
            Ok(_) => break,
            Err(e) => last_error = Some(e),
        }
    }

    last_error.map_or(Ok(()), Err)
}

#[tokio::main]
async fn main() -> ExitCode {
    match main_wrapper().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            if LOGGING_INITIALIZED.load(Ordering::SeqCst) {
                error!("{:?}", anyhow::Error::from(e));
            } else {
                eprintln!("{:?}", anyhow::Error::from(e));
            }
            ExitCode::FAILURE
        }
    }
}
