#[derive(thiserror::Error, Debug, Clone)]
pub enum PushError {
    #[error("packet {seq} arrived too late")]
    Late { seq: u64 },
    #[error("duplicate packet {seq}")]
    Duplicate { seq: u64 },
}

/// The position of a sequence number in the `JitterBuf`.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OutOfWindow {
    /// Sequence number is smaller than the next to be popped sequence number.
    #[error("behind window")]
    Behind,
    /// Sequence number is larger than a full wrap around of the ring buffer subtracting 1.
    /// (`len - 1`)
    #[error("ahead window")]
    Ahead,
}
