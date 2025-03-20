use std::{
    io,
    net::{IpAddr, SocketAddr},
};

use netif::Interface;
use tokio::net::TcpStream;

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
        let ifaces = netif::up().map_err(Error::QueryInterface)?.collect();

        Ok(Self { ifaces })
    }

    pub fn get_addrs_by_name(&self, name: &str) -> Option<Vec<IpAddr>> {
        let mut found = false;

        let addrs = self
            .ifaces
            .iter()
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

    pub async fn get_iface_by_tcp_source_ip(&self, server: SocketAddr) -> Result<&str> {
        let source_ip = Self::get_tcp_source_ip(server).await?;

        self.ifaces
            .iter()
            .find(|iface| *iface.address() == source_ip)
            .map(Interface::name)
            .ok_or(Error::InterfaceNotFound(source_ip))
    }

    async fn get_tcp_source_ip(server: SocketAddr) -> Result<IpAddr> {
        let socket_addr = TcpStream::connect(server)
            .await
            .and_then(|s| s.local_addr())
            .map_err(|e| Error::Connection(server, e))?;
        let ip_addr = match socket_addr {
            SocketAddr::V4(a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(a) => IpAddr::V6(*a.ip()),
        };

        Ok(ip_addr)
    }
}
