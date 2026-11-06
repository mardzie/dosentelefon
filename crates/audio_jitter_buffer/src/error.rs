use std::time::Duration;

#[derive(thiserror::Error, Debug, Clone)]
pub enum PushError {
    #[error("packet {seq} arrived too late")]
    LatePacket { seq: u64 },
    #[error("duplicate packet {seq}")]
    Duplicate { seq: u64 },
    #[error("packet {seq} too far ahead (window ends at {window_end})")]
    TooFarAhead { seq: u64, window_end: u64 },
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum PopError {
    #[error("not ready for {} ms", .0.as_millis())]
    NotReady(Duration),
    #[error("empty frame")]
    Underrun,
}
