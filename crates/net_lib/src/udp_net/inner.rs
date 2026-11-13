use std::{
    io,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

use serde::{Serialize, de::DeserializeOwned};

use super::error::SendError;
use crate::{
    AddrType,
    macros::socket_options,
    udp_net::error::{PeekError, RecvError},
};

#[derive(Debug)]
pub struct Inner<P>
where
    P: Serialize + DeserializeOwned,
{
    socket: UdpSocket,
    addr_type: AddrType,

    packet_phantom_data: PhantomData<P>,
}

impl<P> Inner<P>
where
    P: Serialize + DeserializeOwned,
{
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<(Self, SocketAddr)> {
        let socket = UdpSocket::bind(addr)?;
        let addr = socket.local_addr()?;
        let addr_type = AddrType::from(addr);

        tracing::info!("Bound socket to {}", addr);

        Ok((
            Self {
                socket,
                addr_type,

                packet_phantom_data: PhantomData,
            },
            addr,
        ))
    }

    /// Connects this socket to and remote address.
    ///
    /// [`Inner::send()`], [`Inner::peek()`] and [`Inner::recv()`] will fail when connect was not called beforehand [`Inner::connect()`].
    pub fn connect(&self, addr: impl ToSocketAddrs) -> io::Result<()> {
        self.socket.connect(addr)?;
        tracing::debug!("Connected socket to {}", self.peer_addr()?);

        Ok(())
    }

    /// Send bytes directly to the connected address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to the connected address.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    pub fn send_bytes(&self, buf: &[u8]) -> io::Result<()> {
        self.socket.send(buf)?;

        Ok(())
    }

    /// Send a `P` to the connected address.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    pub fn send(&self, packet: &P, buf: &mut [u8]) -> Result<(), SendError> {
        let slice = postcard::to_slice(packet, buf)?;
        tracing::trace!(bytes = slice.len(), "Sending packet to connected address");
        self.send_bytes(slice)?;

        Ok(())
    }

    /// Send bytes directly to the address.
    ///
    /// The `buf`s content must be able to be turned back into `P` with the `Deserialize` trait or the receiver will get an error.
    ///
    /// This skips the to_bytes call and is useful if the same packet gets sent multiple times to one or more addresses.
    pub fn send_bytes_to(&self, buf: &[u8], addr: impl ToSocketAddrs) -> io::Result<()> {
        self.socket.send_to(buf, addr)?;

        Ok(())
    }

    /// Send a `P` to an address.
    pub fn send_to(
        &self,
        packet: &P,
        addr: impl ToSocketAddrs,
        buf: &mut [u8],
    ) -> Result<(), SendError> {
        let slice = postcard::to_slice(packet, buf)?;
        tracing::trace!(bytes = slice.len(), "Sending packet to address");
        self.send_bytes_to(slice, addr)?;

        Ok(())
    }

    /// Peek a `P` from the connected address.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    pub fn peek(&self, buf: &mut [u8]) -> Result<P, PeekError> {
        let len = self.socket.peek(buf)?;
        tracing::trace!(bytes = len, "Peeked packet from connected address");
        Self::check_for_truncation(&self.addr_type, buf, len)?;
        Ok(postcard::from_bytes(&buf[..len])?)
    }

    /// Peek a `P` from the socket.
    ///
    /// This will not remove the `P` from the sockets received datagrams.
    pub fn peek_from(&self, buf: &mut [u8]) -> Result<(P, SocketAddr), PeekError> {
        let (len, addr) = self.socket.peek_from(buf)?;
        tracing::trace!(bytes = len, from = %addr, "Peeked packet");
        Self::check_for_truncation(&self.addr_type, buf, len)?;
        let packet = postcard::from_bytes(&buf[..len])?;

        Ok((packet, addr))
    }

    /// Receive a `P` from the connected address.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address. This method will fail if the socket is not connected.
    pub fn recv(&self, buf: &mut [u8]) -> Result<P, RecvError> {
        let len = self.socket.recv(buf)?;
        tracing::trace!(bytes = len, "Received packet from connected address");
        Self::check_for_truncation(&self.addr_type, buf, len)?;
        Ok(postcard::from_bytes(&buf[..len])?)
    }

    /// Receive a `P` from the socket.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(P, SocketAddr), RecvError> {
        let (len, addr) = self.socket.recv_from(buf)?;
        tracing::trace!(bytes = len, from = %addr, "Received packet");
        Self::check_for_truncation(&self.addr_type, buf, len)?;
        let packet = postcard::from_bytes(&buf[..len])?;

        Ok((packet, addr))
    }

    /// Returns the local sockets socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Returns the socket address of the remote peer this socket was connected to.
    ///
    /// [`Inner::connect()`] will connect the socket to a remote address.
    /// This method will return an [`std::io::ErrorKind::NotConnected`] error if the socket is not connected.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.socket.peer_addr()
    }

    /// Creates an independent handle to the same socket.
    ///
    /// The returned `Inner` is a reference to the same underlying socket with the same address.
    /// Both handles will read and write the same address and port and options set on one socket will be propagated to the other one.
    pub fn try_clone(&self) -> io::Result<Self> {
        let socket = self.socket.try_clone()?;

        Ok(Self {
            socket,
            addr_type: self.addr_type,

            packet_phantom_data: PhantomData,
        })
    }

    /// Uses the last byte as an indicator that the datagram was truncated.
    /// This does not hold true when the buffer is the `MAX_DATAGRAM_SIZE`
    fn check_for_truncation(
        addr_type: &AddrType,
        buf: &[u8],
        received_len: usize,
    ) -> Result<(), RecvError> {
        // `max_datagram_size` is the maximum size a datagram can have.
        // When the `buf` is that size the truncation check gets disabled and the last byte can be used as a data byte.
        if buf.len() < addr_type.max_datagram_size() && received_len == buf.len() {
            tracing::warn!(
                buf_len = buf.len(),
                received = received_len,
                "Datagram truncated"
            );
            return Err(PeekError::DatagramTruncated);
        }

        Ok(())
    }
}

socket_options!(Inner, socket);

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
    enum Packet {
        Option1,
        Option2,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
    enum BigPacket {
        Option1(u32),
        Option2(u32),
    }

    #[test]
    fn test_inner_connect() {
        let (inner1, addr1, inner2, addr2) = get_inners();
        let mut buf = [0u8; 2];

        inner1.connect(addr2).unwrap();
        inner2.connect(addr1).unwrap();

        let first_packet = Packet::Option1;
        inner1.send(&first_packet, &mut buf).unwrap();

        let first_peeked_packet = inner2.peek(&mut buf).unwrap();
        assert_eq!(first_peeked_packet, first_packet);
        let first_recv_packet = inner2.recv(&mut buf).unwrap();
        assert_eq!(first_recv_packet, first_packet);
        assert_eq!(first_peeked_packet, first_recv_packet);

        let second_packet = Packet::Option2;
        inner2.send(&second_packet, &mut buf).unwrap();

        let second_peeked_packet = inner1.peek(&mut buf).unwrap();
        assert_eq!(second_peeked_packet, second_packet);
        let second_recv_packet = inner1.recv(&mut buf).unwrap();
        assert_eq!(second_recv_packet, second_packet);
        assert_eq!(second_peeked_packet, second_recv_packet);
    }

    #[test]
    fn test_inner_unconnected() {
        let (inner1, addr1, inner2, addr2) = get_inners();
        let mut buf = [0u8; 2];

        let first_packet = Packet::Option1;
        inner1.send_to(&first_packet, addr2, &mut buf).unwrap();

        let (first_peeked_packet, peeked_addr) = inner2.peek_from(&mut buf).unwrap();
        assert_eq!(peeked_addr, addr1);
        assert_eq!(first_peeked_packet, first_packet);
        let (first_recv_packet, addr) = inner2.recv_from(&mut buf).unwrap();
        assert_eq!(addr, addr1);
        assert_eq!(first_recv_packet, first_packet);
        assert_eq!(peeked_addr, addr);
        assert_eq!(first_peeked_packet, first_recv_packet);

        let second_packet = Packet::Option2;
        inner2.send_to(&second_packet, addr1, &mut buf).unwrap();

        let (second_peeked_packet, peeked_addr) = inner1.peek_from(&mut buf).unwrap();
        assert_eq!(peeked_addr, addr2);
        assert_eq!(second_peeked_packet, second_packet);
        let (second_recv_packet, addr) = inner1.recv_from(&mut buf).unwrap();
        assert_eq!(addr, addr2);
        assert_eq!(second_recv_packet, second_packet);
        assert_eq!(peeked_addr, addr);
        assert_eq!(second_peeked_packet, second_recv_packet);
    }

    #[test]
    fn test_inner_truncated() {
        use core::assert_matches;

        let (inner1, addr1, inner2, addr2) = get_inners();

        inner1.connect(addr2).unwrap();
        inner2.connect(addr1).unwrap();

        let mut big_buf = [0u8; 256];
        let first_packet = BigPacket::Option1(100);
        inner1.send(&first_packet, &mut big_buf).unwrap(); // Sufficient large buffer to fit `BigPacket`.

        let mut slightly_tiny_buf = [0u8; 1]; // Slightly too tiny buffer to fit a `BigPacket`.
        let mut fitting_buf = [0u8; 3]; // Exactly fitting buf with +1 space to detect truncation.
        assert_matches!(
            inner2.peek(&mut slightly_tiny_buf),
            Err(PeekError::DatagramTruncated)
        );
        let packet = inner2.recv(&mut fitting_buf).unwrap();
        assert_eq!(packet, first_packet);

        let second_packet = BigPacket::Option2(127);
        inner2.send(&second_packet, &mut big_buf).unwrap();

        let packet = inner1.peek(&mut fitting_buf).unwrap();
        assert_eq!(packet, second_packet);
        assert_matches!(
            inner1.recv(&mut slightly_tiny_buf),
            Err(RecvError::DatagramTruncated)
        );
    }

    fn get_inners<P>() -> (Inner<P>, SocketAddr, Inner<P>, SocketAddr)
    where
        P: Serialize + DeserializeOwned,
    {
        let (inner1, addr1) = Inner::bind("127.0.0.1:0").unwrap();
        let (inner2, addr2) = Inner::bind("127.0.0.1:0").unwrap();

        (inner1, addr1, inner2, addr2)
    }
}
