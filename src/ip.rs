use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// These APIs are still not stabilized, but they're simple enough to just
// reimplement instead of depending on the nightly compiler
// https://github.com/rust-lang/rust/issues/27709
trait Ipv6AddrCompat {
    fn is_unicast_link_local_compat(&self) -> bool;
}

impl Ipv6AddrCompat for Ipv6Addr {
    fn is_unicast_link_local_compat(&self) -> bool {
        (self.segments()[0] & 0xffc0) == 0xfe80
    }
}

// This uses the same heuristics as sssd does for its dyndns functionality

fn is_suitable_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_multicast() && !ip.is_loopback() && !ip.is_link_local() && !ip.is_broadcast()
}

fn is_suitable_ipv6(ip: Ipv6Addr) -> bool {
    !ip.is_unicast_link_local_compat() && !ip.is_loopback() && !ip.is_multicast()
}

pub fn is_suitable_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => is_suitable_ipv4(a),
        IpAddr::V6(a) => is_suitable_ipv6(a),
    }
}
