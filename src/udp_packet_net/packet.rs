use crate::{hash, helpers::VERSION_NUMBER};

pub const HEADER_LEN: usize = 4 + 8 + 4;

#[derive(Debug, Clone, Hash)]
pub struct Packet {
    pub header: Header,
    pub payload: Vec<u8>,
}

/// Header
/// # Protocol
/// | VERSION | TIMESTAMP ms | CHECKSUM | PAYLOAD   |
/// | ------- | ------------ | -------- | --------- |
/// | 4 be    | 8 be in ms  | 4 be     | XXX be    |
///
/// Version is Major * 10^(Minor Places) + Minor.
/// e. g. Major 2; Minor 50;
/// 2 * 10^2 = 200
/// 200 + 50 = 250
/// Version is 250
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Header {
    version: u32,
    timestamp: chrono::DateTime<chrono::Utc>,
    checksum: u32,
}

impl Packet {
    pub fn new(header: Header, payload: Vec<u8>) -> Result<Self, hash::ChecksumMismatch> {
        if header.verify_checksum(&payload) {
            Ok(Self { header, payload })
        } else {
            Err(hash::ChecksumMismatch)
        }
    }

    #[inline]
    pub fn verify_checksum(&self) -> bool {
        self.header.verify_checksum(&self.payload)
    }

    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub fn update_timestamp(&mut self) {
        self.header.update_timestamp();
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn split(self) -> (Header, Vec<u8>) {
        (self.header, self.payload)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut buf = self.payload;
        let len = buf.len();
        buf.resize(HEADER_LEN + buf.len(), 0);
        buf.copy_within(..len, HEADER_LEN);

        self.header.to_bytes(&mut buf[..HEADER_LEN]);

        buf
    }
}

impl From<Vec<u8>> for Packet {
    #[inline]
    fn from(payload: Vec<u8>) -> Self {
        let header = Header::new(&payload);
        Self { header, payload }
    }
}

impl Header {
    pub fn new(payload: &[u8]) -> Self {
        let len = payload.len();
        if len > u16::MAX as usize {
            panic!(
                "Payload too big: Payload must be less or equal to {}",
                u16::MAX
            );
        };

        let checksum = hash::Crc32::checksum(payload);

        Self {
            version: *VERSION_NUMBER,
            timestamp: chrono::Utc::now(),
            checksum,
        }
    }

    #[inline]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[inline]
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    #[inline]
    pub fn update_timestamp(&mut self) {
        self.timestamp = chrono::Utc::now();
    }

    #[inline]
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        hash::Crc32::verify_checksum(payload, self.checksum)
    }

    /// Converts [`Header`] into bytes and write it into `buf`.
    ///
    /// `buf` requires exactly [`HEADER_LEN`] space.
    pub fn to_bytes(&self, buf: &mut [u8]) {
        if buf.len() != HEADER_LEN {
            panic!(
                "`buf` requires exactly `HEADER_LEN` ({}) space.",
                HEADER_LEN
            );
        };

        buf[..4].copy_from_slice(&self.version.to_be_bytes());
        buf[4..12].copy_from_slice(&self.timestamp.timestamp_millis().to_be_bytes());
        buf[12..HEADER_LEN].copy_from_slice(&self.checksum.to_be_bytes());
    }
}

impl From<[u8; HEADER_LEN]> for Header {
    fn from(header_bytes: [u8; HEADER_LEN]) -> Self {
        let mut version = [0u8; 4];
        let mut timestamp = [0u8; 8];
        let mut checksum_bytes = [0u8; 4];

        version.copy_from_slice(&header_bytes[..4]);
        timestamp.copy_from_slice(&header_bytes[4..12]);
        checksum_bytes.copy_from_slice(&header_bytes[12..HEADER_LEN]);
        let checksum = u32::from_be_bytes(checksum_bytes);

        let timestamp_number = i64::from_be_bytes(timestamp);
        let timestamp = match chrono::DateTime::from_timestamp_millis(timestamp_number) {
            Some(timestamp) => timestamp,
            None => {
                tracing::warn!(
                    "Failed to get `DateTime` from {}: Defaulting to UNIX epoch.",
                    timestamp_number
                );
                chrono::DateTime::UNIX_EPOCH
            }
        };

        Self {
            version: u32::from_be_bytes(version),
            timestamp,
            checksum,
        }
    }
}

#[cfg(test)]
mod packet_test {
    use crate::udp_packet_net::packet::{HEADER_LEN, Header, Packet};

    const PAYLOAD: [u8; 6] = [25, 40, 90, 120, 30, 0];

    #[test]
    fn header_to_bytes() {
        let header = get_header();

        let control_bytes = get_header_control_bytes(&header);

        let mut header_bytes = [0u8; HEADER_LEN];
        header.to_bytes(&mut header_bytes);

        assert_eq!(header_bytes, control_bytes);
    }

    #[test]
    fn header_from_bytes() {
        let mut control_header = get_header();
        let mut header_bytes = [0u8; HEADER_LEN];
        control_header.to_bytes(&mut header_bytes);

        let header = Header::from(header_bytes);
        // `Header::from_bytes` uses milliseconds but the original uses the full capabilities of the Computer.
        // They won't match most of the time so set the control to the less exact version.
        control_header.timestamp =
            chrono::DateTime::from_timestamp_millis(control_header.timestamp.timestamp_millis())
                .unwrap();

        assert_eq!(header, control_header);
    }

    #[test]
    fn packet_to_bytes() {
        let header = Header::new(&PAYLOAD);
        let packet = Packet::new(header.clone(), PAYLOAD.to_vec()).unwrap();
        let packet_bytes = packet.into_bytes();

        let mut control_bytes = [0u8; HEADER_LEN + PAYLOAD.len()];
        header.to_bytes(&mut control_bytes[..HEADER_LEN]);
        control_bytes[HEADER_LEN..].copy_from_slice(&PAYLOAD);

        assert_eq!(packet_bytes, control_bytes);
    }

    #[ignore = "Not implemented"]
    #[test]
    fn packet_from_bytes() {
        todo!("Implement packet from bytes");
    }

    #[test]
    fn verify_checksum() {
        let payload = [20, 69, 45, 60, 60, 80, 100];

        let header = Header::new(&payload);

        assert!(header.verify_checksum(&payload));
    }

    fn get_header() -> Header {
        Header {
            version: 1024,
            timestamp: chrono::Utc::now(),
            checksum: 340787525,
        }
    }

    fn get_header_control_bytes(header: &Header) -> [u8; HEADER_LEN] {
        let mut control_bytes = [0u8; HEADER_LEN];
        control_bytes[..4].copy_from_slice(&header.version().to_be_bytes());
        control_bytes[4..12].copy_from_slice(&header.timestamp.timestamp_millis().to_be_bytes());
        control_bytes[12..HEADER_LEN].copy_from_slice(&header.checksum.to_be_bytes());

        control_bytes
    }
}
