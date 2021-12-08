use {
    std::net::{IpAddr, SocketAddr, TcpStream},
    network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to: {0}: {1}")]
    Connection(SocketAddr, std::io::Error),
    #[error("Error when querying network interfaces: {0}")]
    QueryInterface(#[from] network_interface::Error),
    #[error("Could not find interface with IP: {0}")]
    InterfaceNotFound(IpAddr),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Interfaces {
    ifaces: Vec<NetworkInterface>,
}

impl Interfaces {
    pub fn new() -> Result<Self> {
        let ifaces = NetworkInterface::show()?;

        Ok(Self {
            ifaces,
        })
    }

    pub fn get_addrs_by_name(&self, name: &str) -> Option<Vec<IpAddr>> {
        let mut found = false;

        let addrs = self.ifaces.iter()
            .filter(|iface| iface.name == name)
            .inspect(|_| found = true)
            .filter_map(|iface| iface.addr)
            .map(Self::ip_from_addr)
            .collect();

        if found {
            Some(addrs)
        } else {
            None
        }
    }

    pub fn get_iface_by_tcp_source_ip(&self, server: SocketAddr) -> Result<&str> {
        let source_ip = Self::get_tcp_source_ip(server)?;

        self.ifaces.iter().find(|iface| {
            if let Some(addr) = iface.addr {
                if Self::ip_from_addr(addr) == source_ip {
                    return true;
                }
            }

            false
        })
            .map(|iface| iface.name.as_str())
            .ok_or(Error::InterfaceNotFound(source_ip))
    }

    fn ip_from_addr(addr: Addr) -> IpAddr {
        match addr {
            Addr::V4(a) => IpAddr::V4(a.ip),
            Addr::V6(a) => IpAddr::V6(a.ip),
        }
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
