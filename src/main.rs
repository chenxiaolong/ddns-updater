mod config;
mod dns;
mod iface;
mod ip;

use {
    std::{
        borrow::Cow,
        ffi::OsString,
        net::{SocketAddr, ToSocketAddrs},
        path::PathBuf,
        str::FromStr,
        sync::atomic::{AtomicBool, Ordering},
    },
    clap::Parser,
    log::{debug, error, info, trace, warn},
    trust_dns_client::{
        client::Client,
        error::ClientError,
        op::{ResponseCode, UpdateMessage},
        proto::error::ProtoError,
        rr::{
            dnssec::tsig::TSigner,
            DNSClass, Name, RecordType,
        },
    },
    iface::Interfaces,
    crate::dns::DnsClient,
};

// Same as nsupdate
const DEFAULT_FUDGE: u16 = 300;

static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Could not load config: {0:?}: {1}")]
    Config(PathBuf, config::Error),
    #[error("Could not resolve DNS host: {0}: {1}")]
    ResolveDnsHost(String, std::io::Error),
    #[error("Error when querying interfaces: {0}")]
    Interface(#[from] iface::Error),
    #[error("Interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("Hostname has invalid UTF-8: {0:?}")]
    InvalidHostnameUtf8(OsString),
    #[error("Hostname is not a valid DNS name: {0:?}: {1}")]
    InvalidHostnameDns(OsString, ProtoError),
    #[error("Error when querying DNS server: {0}: {1}")]
    DnsClient(SocketAddr, ClientError),
    #[error("Authoritative zone not found: {0}")]
    AuthoritativeZoneNotFound(Name),
    #[error("Failed due to server's error response")]
    ServerResponse,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Parser)]
#[clap(about, author, version)]
struct Opts {
    /// Path to config file
    #[clap(short, long)]
    config: PathBuf,
}

fn update_dns(server: SocketAddr, config: &config::Config) -> Result<()> {
    let ifaces = Interfaces::new()?;
    let iface = match &config.global.interface {
        Some(s) => s.as_str(),
        None => {
            debug!("Autodetecting interface from source IP of TCP connection to {}", server);
            ifaces.get_iface_by_tcp_source_ip(server)?
        }
    };
    debug!("Interface: {:?}", iface);

    let addrs = ifaces.get_addrs_by_name(iface)
        .ok_or_else(|| Error::InterfaceNotFound(iface.to_string()))?;
    let valid_addrs = addrs.into_iter()
        .filter(|ip| ip::is_suitable_ip(*ip))
        .collect::<Vec<_>>();
    debug!("Addresses: {:?}", valid_addrs);

    let name = match &config.global.hostname {
        Some(n) => Cow::Borrowed(n),
        None => {
            let hostname = gethostname::gethostname();
            let hostname_str = hostname.to_str()
                .ok_or_else(|| Error::InvalidHostnameUtf8(hostname.clone()))?;
            let name = Name::from_str(hostname_str)
                .map_err(|e| Error::InvalidHostnameDns(hostname.clone(), e))?;
            Cow::Owned(name)
        }
    };
    debug!("Hostname: {}", name);

    let tsig = TSigner::new(
        config.tsig.secret.clone(),
        config.tsig.algorithm.into(),
        name.clone().into_owned(),
        DEFAULT_FUDGE,
    ).unwrap();

    let client = DnsClient::new(
        server,
        config.global.protocol,
        config.global.timeout.to_duration(),
        tsig,
    ).map_err(|e| Error::DnsClient(server, e))?;

    let zone = match &config.global.zone {
        Some(n) => Cow::Borrowed(n),
        None => {
            debug!("Querying SOA for: {}", name);

            let response = client.query(&name, DNSClass::IN, RecordType::SOA)
                .map_err(|e| Error::DnsClient(server, e))?;
            let authority = response.name_servers();
            if authority.is_empty() {
                return Err(Error::AuthoritativeZoneNotFound(name.into_owned()));
            }

            Cow::Owned(authority[0].name().to_owned())
        }
    };
    debug!("Zone: {}", zone);

    let request = dns::replace_addrs_message(
        zone.clone().into_owned(),
        name.clone().into_owned(),
        config.global.ttl.0,
        &valid_addrs,
    );
    trace!("Update request: {:?}", request);

    for record in request.updates() {
        info!("Record update: {}", record);
    }

    let responses = client.send(request);
    let mut errored = false;

    for response in responses {
        let r = response.map_err(|e| Error::DnsClient(server, e))?;
        trace!("Update response: {:?}", r);

        let code = r.response_code();

        if code != ResponseCode::NoError {
            warn!("Received error response: {0:?} ({0})", code);
            errored = true;
        }
    }

    if errored {
        Err(Error::ServerResponse)
    } else {
        Ok(())
    }
}

fn main_wrapper() -> Result<()> {
    let opts: Opts = Opts::parse();
    let config = config::load_config(&opts.config)
        .map_err(|e| Error::Config(opts.config.clone(), e))?;

    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or(format!(
                "{}={}",
                env!("CARGO_PKG_NAME").replace("-", "_"),
                config.global.log_level,
            ))
    )
        .format_timestamp(None)
        .init();
    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);

    trace!("Loaded config: {:?}", config);

    let server_with_port = match &config.global.name_server {
        h if h.contains(':') => Cow::Borrowed(h.as_str()),
        h => Cow::Owned(format!("{}:{}", h, config.global.protocol.default_port())),
    };
    debug!("Name server: {:?}", server_with_port);

    let servers = server_with_port.to_socket_addrs()
        .map_err(|e| Error::ResolveDnsHost(server_with_port.to_string(), e))?
        .collect::<Vec<_>>();
    debug!("Resolved name servers: {:?}", servers);

    let mut last_error = None;

    for server in servers {
        debug!("Attempting to use name server: {}", server);

        match update_dns(server, &config) {
            Ok(_) => break,
            Err(e) => last_error = Some(e),
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn main() {
    match main_wrapper() {
        Ok(_) => {}
        Err(e) => {
            if LOGGING_INITIALIZED.load(Ordering::SeqCst) {
                error!("{}", e);
            } else {
                eprintln!("{}", e);
            }
            std::process::exit(1);
        }
    }
}
