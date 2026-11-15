//! TCP related types and traits.
//!
//! This module provides TCP networking types.

use std::{
    io::{self, Read, Write},
    marker::PhantomData,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{
    IPV4_HEADER_SIZE,
    macros::{socket_options, tcp_socket_options},
    tcp_net::{
        error::{FrameError, PayloadTooLarge, PostcardFrameError},
        traits::{TcpRead, TcpWrite},
    },
};

pub use listener::TcpListener;
use serde::{Serialize, de::DeserializeOwned};

pub mod error;
pub mod listener;
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

/// Length field size.
pub const HEADER_SIZE: usize = 4;
/// Default max payload size.
pub const DEFAULT_MAX_PAYLOAD_SIZE: u32 = (16 << 20) - HEADER_SIZE as u32; // 16 MiB

#[derive(Debug)]
pub struct TcpNet<T> {
    stream: TcpStream,
    buf: Vec<u8>,
    max_payload_size: u32,

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
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,

            message_phantom: PhantomData,
        }
    }

    fn new_buf() -> Vec<u8> {
        vec![0; MAX_IPV6_SEGMENT_SIZE]
    }

    fn validate_payload(&self, buf: &[u8]) -> Result<(), FrameError> {
        self.validate_payload_len(buf.len())
    }

    fn validate_payload_len(&self, len: usize) -> Result<(), FrameError> {
        if self.payload_len_in_range(len) {
            return Err(FrameError::PayloadTooLarge(PayloadTooLarge {
                len: len,
                max: self.max_payload_size as usize,
            }));
        }

        Ok(())
    }

    fn payload_len_in_range(&self, len: usize) -> bool {
        len + HEADER_SIZE <= self.max_payload_size as usize
    }
}

impl<T> TcpWrite<T> for TcpNet<T>
where
    T: Serialize + DeserializeOwned,
{
    fn write(&mut self, buf: T) -> Result<usize, PostcardFrameError> {
        todo!()
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<usize, FrameError> {
        self.validate_payload(buf)?;
        self.stream.write(buf).map_err(FrameError::from)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), FrameError> {
        self.validate_payload(buf)?;
        self.stream.write_all(buf).map_err(FrameError::from)
    }
}

impl<T> TcpRead<T> for TcpNet<T>
where
    T: Serialize + DeserializeOwned,
{
    fn read(&mut self) -> Result<T, PostcardFrameError> {
        todo!()
    }

    fn read_bytes(&mut self) -> Result<&[u8], FrameError> {
        let mut len_buf = [0u8; (u32::BITS / u8::BITS) as usize];
        self.stream.read_exact(&mut len_buf)?;
        let len_message_field: u32 = u32::from_be_bytes(len_buf);
        let len: usize = len_message_field as usize;
        self.validate_payload_len(len)?;
        self.buf[..len]
    }

    fn read_exact(&mut self) -> Result<&[u8], FrameError> {
        todo!()
    }
}

socket_options!(TcpNet, stream, #[inline]);

tcp_socket_options!(TcpNet, stream, #[inline]);
