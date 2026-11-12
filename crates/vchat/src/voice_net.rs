use std::{collections::VecDeque, io, net::ToSocketAddrs};

use crate::{
    helpers::VERSION_NUMBER,
    udp_packet_net::{self, UdpPacketNet, packet::Packet},
    voice_net::packets::BufferedPacket,
};

pub mod error;
pub(crate) mod packets;

const PACKET_BUF_TIME_IN_QUEUE: chrono::Duration = chrono::Duration::milliseconds(40);

#[derive(Debug)]
pub struct VoiceNet {
    packet_net: UdpPacketNet,

    incoming_packet_buf: VecDeque<BufferedPacket>,
}

impl VoiceNet {
    pub fn new<A>(addr: A) -> Result<Self, io::Error>
    where
        A: ToSocketAddrs,
    {
        let packet_net = UdpPacketNet::new(addr)?;

        Ok(Self {
            packet_net,

            incoming_packet_buf: VecDeque::with_capacity(1024),
        })
    }

    /// Send a packet.
    pub fn send<A>(&self, data: Vec<u8>, addr: &A) -> Result<(), error::SendError>
    where
        A: ToSocketAddrs,
    {
        match self.packet_net.send(Packet::from(data), addr) {
            Ok(_) => Ok(()),
            Err(e) => Err(error::SendError::from(e)),
        }
    }

    /// Tries to receives a packet from queue. If no packet is available `None` is returned.
    pub fn recv(&mut self) -> Option<BufferedPacket> {
        match self.read_packet() {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to read packet: {}", e);
            }
        }

        // Keep `PACKET_BUF_TIME_IN_QUEUE` packets in the queue. This helps to keep as many packets as possible.
        let buf_packet = self.incoming_packet_buf.pop_front()?;
        let now = chrono::Utc::now();
        if now - buf_packet.timestamp() > PACKET_BUF_TIME_IN_QUEUE {
            Some(buf_packet)
        } else {
            self.incoming_packet_buf.push_front(buf_packet);
            None
        }
    }

    /// Tries to read a new packet.
    ///
    /// # Error
    ///
    /// Returns `Err(())` if no valid packet could be read.
    fn read_packet(&mut self) -> Result<(), String> {
        let (packet, src_addr) = match self.packet_net.recv() {
            Ok(packet) => packet,
            Err(e) => match e {
                udp_packet_net::error::RecvError::Io(e) => {
                    return Err(format!(
                        "Failed to receive packet from UDP Packet Net: {}",
                        e
                    ));
                }
                udp_packet_net::error::RecvError::ChecksumMismatch => {
                    return Err("UDP Packet Checksum Mismatch.".to_string());
                }
            },
        };

        // Version
        if packet.header().version() != *VERSION_NUMBER {
            return Err("Version mismatch: Dropping packet.".to_string());
        };

        let packet_timestamp = packet.header().timestamp();

        // Do not insert too old packets.
        let idx = self
            .incoming_packet_buf
            .partition_point(|buf_packet| buf_packet.timestamp() < &packet_timestamp);
        if idx > 0 || self.incoming_packet_buf.is_empty() {
            self.incoming_packet_buf.insert(
                idx,
                BufferedPacket::new(
                    packet_timestamp,
                    crate::voice_net::packets::Packet::new(src_addr, packet.payload),
                ),
            );
            Ok(())
        } else {
            Err(format!(
                "Packet too old: Packet {} < {} Oldest",
                packet_timestamp,
                self.incoming_packet_buf
                    .front()
                    .expect("Has to be Some")
                    .timestamp() // Has to be Some or the If should have been chosen.
            ))
        }
    }
}
