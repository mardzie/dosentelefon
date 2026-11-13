//! # net_lib
//!
//! net_lib is a simple but powerful thin networking crate.

pub mod traits;

mod macros;

/// IPv4 normally is 20 bytes.
const IPV4_HEADER_SIZE: usize = 20;
/// IPv6 has 40 bytes.
const IPV6_HEADER_SIZE: usize = 40;

#[cfg(feature = "tcp")]
pub mod tcp_net;
#[cfg(feature = "udp")]
pub mod udp_net;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AddrType {
    IPv4,
    IPv6,
}

impl From<std::net::SocketAddr> for AddrType {
    fn from(addr: std::net::SocketAddr) -> Self {
        match addr {
            std::net::SocketAddr::V4(_) => Self::IPv4,
            std::net::SocketAddr::V6(_) => Self::IPv6,
        }
    }
}
