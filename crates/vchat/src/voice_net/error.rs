use std::io;

use crate::udp_packet_net;

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("send error: {0}")]
    Io(#[from] io::Error),
    #[error("would block")]
    WouldBlock,
}

impl From<udp_packet_net::error::SendError> for SendError {
    fn from(e: udp_packet_net::error::SendError) -> Self {
        match e {
            udp_packet_net::error::SendError::Io(e) => Self::Io(e),
            udp_packet_net::error::SendError::WouldBlock => Self::WouldBlock,
        }
    }
}
