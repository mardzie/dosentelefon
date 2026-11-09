use std::time::Duration;

use crate::Entry;

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
pub enum PopError<T> {
    #[error("not ready for {} ms", .0.as_millis())]
    NotReady(Duration),
    /// This means the current entry did not get pushed in time.
    ///
    /// This should be signalled to the codec as a missing frame.
    #[error("empty frame")]
    Underrun,
    #[error("uninitialized")]
    Uninitialized,
    /// The sequence number has overflown.
    ///
    /// All remaining `Entry`'s in the buffer are in the `Vec`.
    #[error("sequence number overflow")]
    SeqOverflow(Vec<Entry<T>>),
}
