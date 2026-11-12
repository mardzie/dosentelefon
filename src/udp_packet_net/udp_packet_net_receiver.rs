use std::net::{SocketAddr, UdpSocket};

use crate::udp_packet_net::{
    error,
    packet::{HEADER_LEN, Header, Packet},
};

#[derive(Debug)]
pub struct UdpPacketNetReceiver {
    socket: UdpSocket,
    recv_buf: [u8; u16::MAX as usize],
}

impl UdpPacketNetReceiver {
    pub(super) fn new(socket: UdpSocket, recv_buf: [u8; u16::MAX as usize]) -> Self {
        Self { socket, recv_buf }
    }

    /// Reads a [`Packet`] from stream and returns the `Packet` and the source `SocketAddr`.
    pub fn recv(&mut self) -> Result<(Packet, SocketAddr), error::RecvError> {
        let (len, addr) = match self.socket.recv_from(&mut self.recv_buf) {
            Ok(packet) => packet,
            Err(e) => return Err(error::RecvError::Io(e)),
        };

        // Header
        let mut header_bytes = [0u8; HEADER_LEN];
        header_bytes.copy_from_slice(&self.recv_buf[..HEADER_LEN]);
        let header = Header::from(header_bytes);

        // Header and Payload to bytes and checksum verification.
        let payload_bytes = self.recv_buf[HEADER_LEN..len].to_vec();
        let packet = match Packet::new(header, payload_bytes) {
            Ok(packet) => packet,
            Err(_) => return Err(error::RecvError::ChecksumMismatch),
        };

        Ok((packet, addr))
    }
}
