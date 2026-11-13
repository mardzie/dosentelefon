use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    macros::socket_options,
    udp_net::{UdpReceiver, error, inner::Inner, macros::buf_ops},
};

#[derive(Debug)]
pub struct UdpNetReceiver<P>
where
    P: Serialize + DeserializeOwned,
{
    inner: Inner<P>,
    buf: Vec<u8>,
}

impl<P> UdpNetReceiver<P>
where
    P: Serialize + DeserializeOwned,
{
    pub(super) fn new(inner: Inner<P>, buf: Vec<u8>) -> Self {
        Self { inner, buf }
    }

    /// Connects this socket to and remote address.
    ///
    /// [`UdpNetReceiver::peek()`] and [`UdpNetReceiver::recv()`] will fail when connect was not called beforehand [`UdpNetReceiver::connect()`].
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

impl<P> UdpReceiver<P> for UdpNetReceiver<P>
where
    P: Serialize + DeserializeOwned,
{
    /// Peek a `P` from the connected address.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    ///
    /// [`UdpNetReceiver::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    #[inline]
    fn peek(&mut self) -> Result<P, error::PeekError> {
        self.inner.peek(&mut self.buf)
    }

    /// Peek a `P` from the socket.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    #[inline]
    fn peek_from(&mut self) -> Result<(P, SocketAddr), error::PeekError> {
        self.inner.peek_from(&mut self.buf)
    }

    /// Receive a `P` from the connected address.
    ///
    /// [`UdpNetReceiver::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    #[inline]
    fn recv(&mut self) -> Result<P, error::RecvError> {
        self.inner.recv(&mut self.buf)
    }

    /// Receive a `P` from the socket.
    #[inline]
    fn recv_from(&mut self) -> Result<(P, SocketAddr), error::RecvError> {
        self.inner.recv_from(&mut self.buf)
    }
}

buf_ops!(UdpNetReceiver, buf);

socket_options!(UdpNetReceiver, inner, #[inline]);
