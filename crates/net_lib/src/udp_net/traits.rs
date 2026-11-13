use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::udp_net::error;

pub trait UdpSender<P: Serialize + DeserializeOwned> {
    /// Send bytes directly to the connected address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to the connected address.
    ///
    /// You have to connect the socket to a remote address or this method will fail.
    fn send_bytes(&self, buf: &[u8]) -> io::Result<()>;

    /// Send a `P` to the connected address.
    ///
    /// You have to connect the socket to a remote address or this method will fail.
    fn send(&mut self, packet: &P) -> Result<(), error::SendError>;

    /// Send bytes directly to the address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to one or more addresses.
    fn send_bytes_to(&self, buf: &[u8], addr: impl ToSocketAddrs) -> io::Result<()>;

    /// Send a `P` to an address.
    fn send_to(&mut self, packet: &P, addr: impl ToSocketAddrs) -> Result<(), error::SendError>;
}

pub trait UdpReceiver<P: Serialize + DeserializeOwned> {
    /// Peek a `P` from the connected address.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    ///
    /// You have to connect the socket to a remote address or this method will fail.
    fn peek(&mut self) -> Result<P, error::PeekError>;

    /// Peek a `P` from the socket.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    fn peek_from(&mut self) -> Result<(P, SocketAddr), error::PeekError>;

    /// Receive a `P` from the connected address.
    ///
    /// You have to connect the socket to a remote address or this method will fail.
    fn recv(&mut self) -> Result<P, error::RecvError>;

    /// Receive a `P` from the socket.
    fn recv_from(&mut self) -> Result<(P, SocketAddr), error::RecvError>;
}

pub trait BufOps {
    /// Returns the current buffer length.
    fn buf_len(&self) -> usize;

    /// Resizes the buffer to the set length and tries to fit the allocation as much as possible.
    fn resize_buf(&mut self, new_len: usize);
}
