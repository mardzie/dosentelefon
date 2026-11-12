use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const IPV4_BYTES_COUNT: usize = 4;
pub const IPV6_BYTES_COUNT: usize = 16;
pub const PORT_BYTES_COUNT: usize = 2;
pub const IPV4_ADDR_BYTES_COUNT: usize = IPV4_BYTES_COUNT + PORT_BYTES_COUNT;
pub const IPV6_ADDR_BYTES_COUNT: usize = IPV6_BYTES_COUNT + PORT_BYTES_COUNT;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FriendCode(SocketAddr);

impl FriendCode {
    pub fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }

    pub fn new_local(port: u16) -> Result<Self, IpError> {
        let ip = match local_ip_address::local_ip() {
            Ok(ip) => ip,
            Err(e) => {
                tracing::warn!("Failed to get local ip: {}", e);
                return Err(IpError::from(e));
            }
        };

        tracing::trace!("Created new local `FriendCode`.");

        Ok(Self::from_ip_port(ip, port))
    }

    pub fn new_public(runtime: &tokio::runtime::Runtime, port: u16) -> Result<Self, IpError> {
        let ip = match runtime.block_on(public_ip_address::perform_lookup(None)) {
            Ok(lookup) => lookup.ip,
            Err(e) => {
                tracing::warn!("Failed to perform public ip lookup.");
                return Err(IpError::from(e));
            }
        };

        tracing::trace!("Created new public `FriendCode`.");

        Ok(Self::from_ip_port(ip, port))
    }

    pub fn from_ip_port(ip: IpAddr, port: u16) -> Self {
        Self::new(SocketAddr::new(ip, port))
    }

    /// Takes a hexadecimal encoded binary `SocketAddr`.
    ///
    /// Valid characters include all hexadecimal digits, as well as `.`, `:` and whitespaces.
    pub fn from_string_friend_code(fc_string: &str) -> Result<Self, InvalidFriendCodeString> {
        let fc_string = fc_string
            .split_whitespace()
            .collect::<String>()
            .replace(".", "")
            .replace(":", "");
        let bytes = hex::decode(&fc_string).map_err(|_| InvalidFriendCodeString)?;

        let mut port_bytes = [0u8; 2];
        port_bytes.copy_from_slice(&bytes[bytes.len() - 2..]);
        let port = u16::from_be_bytes(port_bytes);

        let ip = match bytes.len() {
            IPV4_ADDR_BYTES_COUNT => {
                let mut octets = [0u8; 4];
                octets.copy_from_slice(&bytes[..4]);
                let bits = u32::from_be_bytes(octets);
                IpAddr::V4(Ipv4Addr::from_bits(bits))
            }
            IPV6_ADDR_BYTES_COUNT => {
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&bytes[..16]);
                let bits = u128::from_be_bytes(octets);
                IpAddr::V6(Ipv6Addr::from_bits(bits))
            }
            _ => return Err(InvalidFriendCodeString),
        };

        tracing::trace!("Created `FriendCode` from String.");

        Ok(Self::new(SocketAddr::new(ip, port)))
    }

    pub fn to_socket_addr(&self) -> &SocketAddr {
        &self.0
    }

    pub fn into_socket_addr(self) -> SocketAddr {
        self.0
    }

    pub fn to_string(&self) -> String {
        let bytes = match self.0.ip() {
            IpAddr::V4(v4) => {
                let mut bytes = vec![0u8; IPV4_ADDR_BYTES_COUNT];
                bytes[..4].copy_from_slice(&v4.to_bits().to_be_bytes());
                bytes[4..].copy_from_slice(&self.0.port().to_be_bytes());

                bytes
            }
            IpAddr::V6(v6) => {
                let mut bytes = vec![0u8; IPV6_ADDR_BYTES_COUNT];
                bytes[..16].copy_from_slice(&v6.to_bits().to_be_bytes());
                bytes[16..].copy_from_slice(&self.0.port().to_be_bytes());

                bytes
            }
        };

        hex::encode(bytes)
    }

    pub fn to_pretty_string(&self) -> String {
        self.to_string()
            .to_uppercase()
            .chars()
            .collect::<Vec<char>>()
            .chunks(2)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl From<SocketAddr> for FriendCode {
    fn from(addr: SocketAddr) -> Self {
        Self::new(addr)
    }
}

impl From<FriendCode> for SocketAddr {
    fn from(friend_code: FriendCode) -> Self {
        friend_code.into_socket_addr()
    }
}

impl TryFrom<&str> for FriendCode {
    type Error = InvalidFriendCodeString;

    fn try_from(fc: &str) -> Result<Self, Self::Error> {
        FriendCode::from_string_friend_code(fc)
    }
}

impl std::fmt::Display for FriendCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(thiserror::Error, Debug, Clone)]
#[error("invalid friend code string")]
pub struct InvalidFriendCodeString;

#[derive(thiserror::Error, Debug)]
pub enum IpError {
    #[error("local ip error: {0}")]
    Local(#[from] local_ip_address::Error),
    #[error("public ip error: {0}")]
    Public(#[from] public_ip_address::error::Error),
}

#[cfg(test)]
mod friend_code_test {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use crate::FriendCode;

    #[test]
    fn from_string_friend_code_ipv4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 69);
        let friend_code_addr = FriendCode::new(addr);

        let fc = "7F 00 00 01 00 45".to_string();
        assert_eq!(fc, friend_code_addr.to_pretty_string());

        let friend_code = FriendCode::from_string_friend_code(&fc).unwrap();
        assert_eq!(&addr, friend_code.to_socket_addr());
    }

    #[test]
    fn from_string_friend_code_ipv6() {
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 5000);
        let friend_code_addr = FriendCode::new(addr);

        let fc = "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 13 88".to_string();
        assert_eq!(fc, friend_code_addr.to_pretty_string());

        let friend_code = FriendCode::from_string_friend_code(&fc).unwrap();
        assert_eq!(&addr, friend_code.to_socket_addr());
    }
}
