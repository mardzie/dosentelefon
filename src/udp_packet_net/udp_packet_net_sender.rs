use std::net::{ToSocketAddrs, UdpSocket};

use crate::udp_packet_net::{error, packet::Packet};

#[derive(Debug)]
pub struct UdpPacketNetSender {
    socket: UdpSocket,
}

impl UdpPacketNetSender {
    pub(super) fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }

    /// Sends the `packet` to the given address.
    pub fn send<A>(&self, packet: Packet, addr: A) -> Result<usize, error::SendError>
    where
        A: ToSocketAddrs,
    {
        self.socket
            .send_to(&packet.into_bytes(), addr)
            .map_err(error::SendError::Io)
    }
}
