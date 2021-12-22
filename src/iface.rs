use {
    std::{
        io,
        net::{IpAddr, SocketAddr, TcpStream},
    },
    netif::Interface,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to: {0}: {1}")]
    Connection(SocketAddr, io::Error),
    #[error("Error when querying network interfaces: {0}")]
    QueryInterface(io::Error),
    #[error("Could not find interface with IP: {0}")]
    InterfaceNotFound(IpAddr),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Interfaces {
    ifaces: Vec<Interface>,
}

impl Interfaces {
    pub fn new() -> Result<Self> {
        let ifaces = netif::all()
            .map_err(Error::QueryInterface)?
            .collect();

        Ok(Self {
            ifaces,
        })
    }

    pub fn get_addrs_by_name(&self, name: &str) -> Option<Vec<IpAddr>> {
        let mut found = false;

        let addrs = self.ifaces.iter()
            .filter(|iface| iface.name() == name)
            .inspect(|_| found = true)
            .map(|iface| *iface.address())
            .collect();

        if found {
            Some(addrs)
        } else {
            None
        }
    }

    pub fn get_iface_by_tcp_source_ip(&self, server: SocketAddr) -> Result<&str> {
        let source_ip = Self::get_tcp_source_ip(server)?;

        self.ifaces.iter()
            .find(|iface| *iface.address() == source_ip)
            .map(|iface| iface.name())
            .ok_or(Error::InterfaceNotFound(source_ip))
    }

    fn get_tcp_source_ip(server: SocketAddr) -> Result<IpAddr> {
        let socket_addr = TcpStream::connect(server)
            .and_then(|s| s.local_addr())
            .map_err(|e| Error::Connection(server, e))?;
        let ip_addr = match socket_addr {
            SocketAddr::V4(a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(a) => IpAddr::V6(*a.ip()),
        };

        Ok(ip_addr)
    }
}
