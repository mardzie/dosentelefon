#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Packet {
    src_addr: std::net::SocketAddr,
    payload: Vec<u8>,
}

impl Packet {
    #[inline]
    pub fn new(src_addr: std::net::SocketAddr, payload: Vec<u8>) -> Self {
        Self { src_addr, payload }
    }

    #[inline]
    pub fn src_addr(&self) -> &std::net::SocketAddr {
        &self.src_addr
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[inline]
    pub fn inner(self) -> (std::net::SocketAddr, Vec<u8>) {
        (self.src_addr, self.payload)
    }
}

impl From<BufferedPacket> for Packet {
    #[inline]
    fn from(buf_packet: BufferedPacket) -> Self {
        buf_packet.packet
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferedPacket {
    timestamp: chrono::DateTime<chrono::Utc>,
    packet: Packet,
}

impl BufferedPacket {
    #[inline]
    pub fn new(timestamp: chrono::DateTime<chrono::Utc>, packet: Packet) -> Self {
        Self { timestamp, packet }
    }

    #[inline]
    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.timestamp
    }

    #[inline]
    pub fn inner(self) -> (chrono::DateTime<chrono::Utc>, Packet) {
        (self.timestamp, self.packet)
    }
}
