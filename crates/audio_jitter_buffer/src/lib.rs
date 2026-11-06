use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use audio_common::frame_size::FrameSize;
pub use entry::Entry;

use crate::error::{PopError, PushError};

mod entry;

pub mod error;

pub type SequenceNumber = u64;

/// The `JitterBuf` handles the timing of pushed entries.
/// `target_delay` is the delay that each entry must at least stay inside the buffer.
/// This is done to wait for incoming packets so that a stream does not play newer samples than the current if they are still on the wire.
///
/// The `JitterBuf` is implemented as a ring buffer over `Option<Entry<T>>`.
#[derive(Debug)]
pub struct JitterBuf<T> {
    inner: Vec<Option<Entry<T>>>,
    /// The biggest pushed sequence number.
    ///
    /// Not all slots between `pop_seq` and `push_seq` must be filled.
    push_seq: SequenceNumber,
    /// The next sequence number to be popped.
    pop_seq: SequenceNumber,
    /// The time when a new entry can get popped.
    deadline: Option<Instant>,
    /// The currently stored `Entry` count.
    len: usize,

    target_delay: Duration,
    frame_size: FrameSize,
}

impl<T> JitterBuf<T> {
    const DEFAULT_CAPACTITY_MULTIPLIER: NonZeroUsize =
        NonZeroUsize::new(4).expect("default capacity multiplier should be non zero");

    /// Constructs a new `JitterBuf`.
    ///
    /// `target_delay` is the delay that is kept between pushing a entry and popping it again.
    ///
    /// `frame_size` is `FrameSize` and is used to calculate how many packets have to be kept in the buffer.
    /// `target_delay.div_ceil(frame_size.duration())` -> `100 ms / 20 ms = 5 packets`
    /// (The result will be multiplied by the `DEFAULT_CAPACITY_MULTIPLIER` to acomodate bursts of packets or if packets got popped late.)
    #[must_use]
    pub fn new(target_delay: Duration, frame_size: FrameSize) -> Self {
        Self::with_multiplier(target_delay, frame_size, Self::DEFAULT_CAPACTITY_MULTIPLIER)
    }

    /// Constructs a new `JitterBuf`.
    ///
    /// `target_delay` is the delay that is kept between pushing a entry and popping it again.
    ///
    /// `frame_size` is `FrameSize` and is used to calculate how many packets have to be kept in the buffer.
    /// `target_delay.div_ceil(frame_size.duration())` -> `100 ms / 20 ms = 5 packets`
    /// The result will be multiplied by the `capacity_multiplier` to acomodate bursts of packets or if packets got popped late.
    pub fn with_multiplier(
        target_delay: Duration,
        frame_size: FrameSize,
        // To multiply the necessary capacity for bursts of incoming packets.
        capacity_multiplier: NonZeroUsize,
    ) -> Self {
        let capacity = target_delay
            .as_nanos()
            .div_ceil(frame_size.duration().as_nanos());
        let n = capacity as usize * capacity_multiplier.get();

        Self {
            inner: std::iter::repeat_with(|| None).take(n).collect(),
            pop_seq: 0,
            push_seq: 0,
            deadline: None,
            len: 0,

            target_delay,
            frame_size,
        }
    }

    /// Push a `Entry` to the `JitterBuf`.
    ///
    /// If a sequence number is pushed multiple times only the first gets accepted, other will get rejected with [`PushError::Duplicate`].
    ///
    /// If the push side of the ring buffer touches the pop side then [`PushError::TooFarAhead`] gets returned.
    /// This can be fixed if a entry gets popped, then there is space for a new push.
    ///
    /// A entry with a sequence number smaller than the last popped one errors with [`PushError::LatePacket`].
    ///
    /// Initializes the `JitterBuf` on first call.
    #[inline]
    pub fn push(&mut self, entry: Entry<T>) -> Result<(), PushError> {
        let seq = entry.seq();

        if seq < self.pop_seq {
            return Err(PushError::LatePacket { seq });
        }

        let index = self.index_from_seq(seq);
        let value = self.inner.get_mut(index).expect("index must be in bounds");
        if let Some(old_entry) = value {
            if old_entry.seq() == seq {
                return Err(PushError::Duplicate { seq });
            } else {
                return Err(PushError::TooFarAhead {
                    seq,
                    window_end: self.pop_seq(),
                });
            }
        }

        *value = Some(entry);
        self.push_seq = self.push_seq.max(seq);
        self.len += 1;
        if self.deadline.is_none() {
            self.deadline = Some(
                Instant::now()
                    .checked_add(self.target_delay.checked_add(self.frame_size.duration())
                        .expect("`target_delay` and `frame_size.duration()` should not be overflowing addition"))
                    .expect("`target_delay + frame_size` should not overflow `Instant::now()`"),
            );
        }

        Ok(())
    }

    /// Pop the next `Entry` from the `JitterBuf`.
    ///
    /// `PopError::NotReady` is returned if the `target_delay` didn't yet pass.
    ///
    /// `PopError::Underrun` is returned if a mature slot has no entry present.
    /// Depending on the codec this should be signalled to the codec.
    ///
    /// `PopError::Uninitialized` is returned if the `JitterBuf` has not been initialized by a `JitterBuf::push`.
    /// A call to [`JitterBuf::push`] will initialize the `JitterBuf`.
    #[inline]
    pub fn pop(&mut self) -> Result<Entry<T>, PopError> {
        let deadline = match self.deadline {
            Some(deadline) => deadline,
            None => return Err(PopError::Uninitialized),
        };
        let now = Instant::now();
        let time_to_maturity = deadline.checked_duration_since(now);
        if let Some(duration) = time_to_maturity {
            return Err(PopError::NotReady(duration));
        }
        self.deadline = deadline
            .checked_add(self.frame_size.duration())
            .expect("no system should ever be running for this long")
            .into();

        let index = self.index_from_seq(self.pop_seq());
        let value = self
            .inner
            .get_mut(index)
            .expect("index should be in bounds");
        self.pop_seq += 1;
        match value.take() {
            Some(entry) => {
                debug_assert_eq!(entry.seq(), self.pop_seq());
                self.len -= 1;
                Ok(entry)
            }
            None => Err(PopError::Underrun),
        }
    }

    /// The biggest pushed sequence number.
    ///
    /// Not all slots between `pop_seq` and `push_seq` must be filled.
    #[must_use]
    #[inline]
    pub fn push_seq(&self) -> u64 {
        self.push_seq
    }

    /// The next sequence number to be popped.
    #[must_use]
    #[inline]
    pub fn pop_seq(&self) -> u64 {
        self.pop_seq
    }

    /// The current sequence number window length.
    #[must_use]
    #[inline]
    pub fn window_len(&self) -> usize {
        usize::try_from(self.push_seq() - self.pop_seq())
            .expect("`push_seq + pop_seq` should be convertible to `usize`")
    }

    /// The currently stored `Entry` count.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the index of the `sequence_number`.
    ///
    /// The index is always valid.
    fn index_from_seq(&self, seq: u64) -> usize {
        let index = usize::try_from(seq).expect("can not convert u64 to usize") % self.inner.len();
        debug_assert!(self.inner.get(index).is_some());

        index
    }
}

impl<T> Default for JitterBuf<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new(Duration::from_millis(100), FrameSize::default())
    }
}

#[cfg(test)]
mod jitter_buf_test {
    use std::time::{Duration, Instant};

    use audio_common::frame_size::FrameSize;
    use tracing::info;

    use crate::{Entry, JitterBuf, error::PopError};

    #[test]
    fn test_push() {
        logging_setup();

        const N: usize = 4;
        let now = Instant::now();
        let mut entries = Vec::with_capacity(N);
        for i in 0..N {
            let timestamp = now
                .checked_add(Duration::from_millis(20 * i as u64))
                .unwrap();
            entries.push(Entry::new(i as u64, timestamp, i + 1));
        }

        let target_delay = Duration::from_millis(100);
        let mut jb = JitterBuf::new(target_delay, FrameSize::default());
        for entry in entries.clone() {
            info!(
                "Pushing entry with seq: {}, timestamp {:?}, inner: {}",
                entry.seq(),
                entry.timestamp(),
                entry.get()
            );
            jb.push(entry).unwrap();
        }

        for (i, opt_entry) in jb.inner[..N].iter().enumerate() {
            let entry = opt_entry
                .as_ref()
                .expect("entry should have been written to this index");
            assert_eq!(
                entry, &entries[i],
                "Entry {:?} at {} should be {:?}",
                entry, i, entries[i]
            )
        }
        assert_eq!(
            jb.push_seq(),
            u64::try_from(N - 1).expect("should be convertible to `u64`")
        );
    }

    #[test]
    fn test_delay() {
        logging_setup();

        let target_delay = Duration::from_millis(100);
        let mut jb: JitterBuf<i32> = JitterBuf::new(target_delay, FrameSize::default());

        let now = Instant::now();
        let entry = Entry::new(0, now, 32);
        jb.push(entry.clone()).unwrap();

        let (popped, timestamp) = loop {
            match jb.pop() {
                Ok(entry) => break (entry, Instant::now()),
                Err(PopError::Underrun) => panic!("Can not underrun"),
                Err(PopError::NotReady(dur)) => {
                    assert!(
                        dur >= target_delay,
                        "duration to ready must be smaller than the target delay"
                    )
                }
                Err(PopError::Uninitialized) => panic!("Should be initialized"),
            };
        };

        assert_eq!(popped, entry);
        assert!(timestamp.duration_since(now) > jb.target_delay + jb.frame_size.duration());
    }

    fn logging_setup() {
        tracing_subscriber::fmt::init();
    }
}
