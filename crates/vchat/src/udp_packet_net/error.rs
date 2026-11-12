use std::io;

#[derive(thiserror::Error, Debug)]
pub enum SendError {
    #[error("Send IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Would Block")]
    WouldBlock,
}

#[derive(thiserror::Error, Debug)]
pub enum RecvError {
    #[error("recv io error: {0}")]
    Io(#[from] io::Error),
    #[error("checksum mismatch")]
    ChecksumMismatch,
}
