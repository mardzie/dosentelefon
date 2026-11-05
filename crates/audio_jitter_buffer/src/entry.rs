use std::time::Instant;

use crate::SequenceNumber;

/// A `JitterBuf` entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry<T> {
    seq: SequenceNumber,
    /// The arrival timestamp.
    timestamp: Instant,
    inner: T,
}

impl<T> Entry<T> {
    /// `seq` is the sequence number that is used to detect missing packets. `seq > 0`
    ///
    /// `timestamp` is the timestamp at receive time.
    ///
    /// `inner` is the actual data.
    #[must_use]
    #[inline]
    pub fn new(seq: u64, timestamp: Instant, inner: T) -> Self {
        Self {
            seq,
            timestamp,
            inner,
        }
    }

    /// Read [`Entry::new()`] to learn more.
    ///
    /// `timestamp` is the current time at call.
    #[must_use]
    #[inline]
    pub fn from_now(seq: u64, inner: T) -> Self {
        Self::new(seq, Instant::now(), inner)
    }

    /// The sequence number of this `Entry`.
    #[must_use]
    #[inline]
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// The timestamp of this `Entry`.
    #[must_use]
    #[inline]
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Get a reference to `T`.
    #[must_use]
    #[inline]
    pub fn get(&self) -> &T {
        &self.inner
    }

    #[must_use]
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(test)]
mod entry_test {
    use std::time::Instant;

    use crate::Entry;

    #[test]
    fn field_access() {
        let seq = 1;
        let timestamp = Instant::now();
        let inner: u128 = 200;
        let entry = Entry::new(seq, timestamp, inner);

        assert_eq!(entry.seq(), seq);
        assert_eq!(entry.timestamp(), timestamp);
        assert_eq!(entry.get(), &inner);
        assert_eq!(entry.into_inner(), inner);
    }
}
