use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

pub mod error;
pub mod packet;
pub mod udp_packet_net_receiver;
pub mod udp_packet_net_sender;

use packet::{HEADER_LEN, Packet};

use crate::udp_packet_net::{
    udp_packet_net_receiver::UdpPacketNetReceiver, udp_packet_net_sender::UdpPacketNetSender,
};

pub const MAX_PACKET_SIZE: usize = 512;
/// The max payload size is 512 bytes.
///
/// This is to maximize throughput and minimize latency and bytes lost.
pub const MAX_PAYLOAD_SIZE: usize = MAX_PACKET_SIZE - HEADER_LEN;

/// The UDP Socket handler.
///
/// There must never exists two identical `SocketAddr` in `addresses`!
#[derive(Debug)]
pub struct UdpPacketNet {
    sender: UdpPacketNetSender,
    receiver: UdpPacketNetReceiver,
}

impl UdpPacketNet {
    pub fn new<A>(addr: A) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        let socket = UdpSocket::bind(addr)?;
        let socket_c = socket.try_clone()?;

        Ok(Self {
            sender: UdpPacketNetSender::new(socket),
            receiver: UdpPacketNetReceiver::new(socket_c, [0u8; u16::MAX as usize]),
        })
    }

    /// Sends the `packet` to the given address.
    pub fn send<A>(&self, packet: Packet, addr: A) -> Result<usize, error::SendError>
    where
        A: ToSocketAddrs,
    {
        self.sender.send(packet, addr)
    }

    /// Reads a [`Packet`] from stream and returns the `Packet` and the source `SocketAddr`.
    pub fn recv(&mut self) -> Result<(Packet, SocketAddr), error::RecvError> {
        self.receiver.recv()
    }

    /// Returns the sending and reading halves.
    pub fn split(self) -> (UdpPacketNetSender, UdpPacketNetReceiver) {
        (self.sender, self.receiver)
    }
}
