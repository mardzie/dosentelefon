use std::fmt::Display;

#[derive(Debug)]
pub struct Crc32;

impl Crc32 {
    /// Create a checksum with big endianess.
    pub fn checksum(data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }

    pub fn verify_checksum(data: &[u8], checksum: u32) -> bool {
        Self::checksum(data) == checksum
    }
}

#[derive(Debug)]
pub struct ChecksumMismatch;

impl Display for ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Checksum Mismatch")
    }
}
