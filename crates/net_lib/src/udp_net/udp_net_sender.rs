use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    macros::socket_options,
    udp_net::{UdpSender, error, inner::Inner, macros::buf_ops},
};

#[derive(Debug)]
pub struct UdpNetSender<P>
where
    P: Serialize + DeserializeOwned,
{
    inner: Inner<P>,
    buf: Vec<u8>,
}

impl<P> UdpNetSender<P>
where
    P: Serialize + DeserializeOwned,
{
    pub(super) fn new(inner: Inner<P>, buf: Vec<u8>) -> Self {
        Self { inner, buf }
    }

    /// Connects this socket to and remote address.
    ///
    /// [`UdpNetSender::send()`] will fail when connect was not called beforehand [`UdpNetSender::connect()`].
    #[inline]
    pub fn connect(&self, addr: impl ToSocketAddrs) -> io::Result<()> {
        self.inner.connect(addr)
    }

    /// Returns the local sockets socket address.
    #[inline]
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address.
    /// This method will return an [`std::io::ErrorKind::NotConnected`] error if the socket is not connected.
    #[inline]
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }
}

impl<P> UdpSender<P> for UdpNetSender<P>
where
    P: Serialize + DeserializeOwned,
{
    /// Send bytes directly to the connected address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to the connected address.
    ///
    /// [`UdpNetSender::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    #[inline]
    fn send_bytes(&self, buf: &[u8]) -> io::Result<()> {
        self.inner.send_bytes(buf)
    }

    /// Send a `P` to the connected address.
    ///
    /// [`UdpNetSender::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    #[inline]
    fn send(&mut self, packet: &P) -> Result<(), error::SendError> {
        self.inner.send(packet, &mut self.buf)?;

        Ok(())
    }

    /// Send bytes directly to the address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to one or more addresses.
    #[inline]
    fn send_bytes_to(&self, buf: &[u8], addr: impl ToSocketAddrs) -> io::Result<()> {
        self.inner.send_bytes_to(buf, addr)
    }

    /// Send a `P` to an address.
    #[inline]
    fn send_to(&mut self, packet: &P, addr: impl ToSocketAddrs) -> Result<(), error::SendError> {
        self.inner.send_to(packet, addr, &mut self.buf)?;

        Ok(())
    }
}

buf_ops!(UdpNetSender, buf, false);

socket_options!(UdpNetSender, inner, #[inline]);
