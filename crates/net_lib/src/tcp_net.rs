//! TCP related types and traits.
//!
//! This module provides TCP networking types.

use std::{
    io,
    marker::PhantomData,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{IPV4_HEADER_SIZE, macros::socket_options};

pub use listener::TcpListener;
use serde::{Serialize, de::DeserializeOwned};

pub mod listener;
pub mod message;
pub mod traits;

pub const TCP_MESSAGE_VERSION: u8 = 0;

const TCP_HEADER_SIZE: usize = 20;
/// The maximum IPv4 payload size.
///
/// Includes the TCP header and IPv4 header
const MAX_IPV4_SEGMENT_SIZE: usize = u16::MAX as usize - TCP_HEADER_SIZE - IPV4_HEADER_SIZE;
/// The maximum IPv6 payload size.
///
/// Includes the TCP header but excludes the IPv6 header.
const MAX_IPV6_SEGMENT_SIZE: usize = u16::MAX as usize - TCP_HEADER_SIZE;

#[derive(Debug)]
pub struct TcpNet<T> {
    stream: TcpStream,
    buf: Vec<u8>,
    state: State,

    message_phantom: PhantomData<T>,
}

impl<T> TcpNet<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn connect<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr)?;

        Ok(Self::from_stream(stream))
    }

    pub fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        let stream = TcpStream::connect_timeout(addr, timeout)?;
        Ok(Self::from_stream(stream))
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            stream,
            buf: Self::new_buf(),
            message_phantom: PhantomData,
        }
    }

    fn new_buf() -> Vec<u8> {
        vec![0; MAX_IPV6_SEGMENT_SIZE]
    }
}

socket_options!(TcpNet, stream, #[inline]);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
    #[default]
    Handshake,
    AwaitingHandshakeAccept,
    Data,
}
