#![warn(clippy::pedantic)]

use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use audio_common::frame_size::FrameSize;
pub use entry::Entry;

use crate::error::{OutOfWindow, PopError, PushError};

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
    slots: Vec<Option<Entry<T>>>,
    /// Next slot to release. Anything below it is late.
    next_seq: SequenceNumber,
    /// One past the newest buffered frame.
    /// Invariant: `next_seq <= end <= next_seq + N`.
    end: SequenceNumber,
    /// The currently stored `Entry` count.
    len: usize,
    state: State,

    delay: Duration,
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
    ///
    /// # Panic
    ///
    /// Panics when the division of `target_delay` and `frame_size.duration()` nanos times the multiplier does not fit into a `usize`.
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
    ///
    /// # Panics
    ///
    /// Panics when the division of `target_delay` and `frame_size.duration()` nanos times the multiplier does not fit into a `usize`.
    #[must_use]
    pub fn with_multiplier(
        delay: Duration,
        frame_size: FrameSize,
        // To multiply the necessary capacity for bursts of incoming packets.
        capacity_multiplier: NonZeroUsize,
    ) -> Self {
        let capacity = delay.as_nanos().div_ceil(frame_size.duration().as_nanos()) + 2;
        let n = usize::try_from(capacity)
            .ok()
            .and_then(|c| c.checked_mul(capacity_multiplier.get()))
            .expect("`usize` overflow");

        Self {
            slots: std::iter::repeat_with(|| None).take(n).collect(),
            next_seq: 0,
            end: 0,
            len: 0,
            state: State::Idle,

            delay,
            frame_size,
        }
    }

    /// Reset the `JitterBuf` into it's starting state.
    #[inline]
    pub fn reset(&mut self) -> Vec<Entry<T>> {
        self.next_seq = 0;
        self.end = 0;
        self.len = 0;
        self.state = State::Idle;
        self.slots.iter_mut().filter_map(|e| e.take()).collect()
    }

    /// Puts a received frame into its slot.
    ///
    /// # Errors
    ///
    /// Returns a [`PushError`] and `Entry` when its late or a duplicate.
    #[inline]
    pub fn push(&mut self, entry: Entry<T>) -> Result<(), (PushError, Entry<T>)> {
        let seq = entry.seq();
        let index = match self.slot_index(seq) {
            Ok(index) => index,
            Err(OutOfWindow::Behind) => return Err((PushError::Late { seq }, entry)),
            Err(OutOfWindow::Ahead) => {
                // Too far ahead. Slide window forward dropping the oldest frames.
                let new_start = seq + 1 - self.capacity() as u64;
                for s in self.next_seq..new_start.min(self.end) {
                    let index = self.wrapping_index(s);
                    let slot = self
                        .slots
                        .get_mut(index)
                        .expect("index should be always valid");
                    *slot = None;
                }
                self.next_seq = new_start;
                self.wrapping_index(seq)
            }
        };

        let slot = self.slots.get_mut(index).expect("index must be in bounds");
        if slot.is_some() {
            return Err((PushError::Duplicate { seq }, entry));
        }
        if matches!(self.state, State::Idle) {
            self.state = State::Buffering {
                start: entry.timestamp() + self.delay,
            };
        }
        *slot = Some(entry);
        self.end = self.end.max(seq + 1);
        self.len += 1;
        Ok(())
    }

    /// Release the next slot if its due at `now`.
    ///
    /// Call in a loop until `None` to drain everything that is due.
    #[inline]
    pub fn pop(&mut self, now: Instant) -> Result<Slot<T>, PopError> {
        let due = match self.state {
            State::Idle => return Err(PopError::Empty),
            State::Buffering { start: due } | State::Playing { due } => due,
        };
        if now < due {
            return Err(PopError::Early { due });
        }
        if self.next_seq == self.end {
            // Underrun
            // Sender paused or network stalled.
            // Start buffering again with next push.
            self.state = State::Idle;
            return Err(PopError::Empty);
        }
        if matches!(self.state, State::Buffering { .. }) {
            // Playback (re)starts at the oldest buffered frame.
            // Terminates: frame `end - 1` is always buffered.
            while self
                .slots
                .get(self.wrapping_index(self.next_seq))
                .expect("index should always be valid")
                .is_none()
            {
                self.next_seq += 1;
            }
        }

        let seq = self.next_seq;
        self.next_seq += 1;
        self.len -= 1;
        self.state = State::Playing {
            due: due + self.frame_size.duration(),
        };

        let index = self.wrapping_index(seq);
        Ok(
            match self
                .slots
                .get_mut(index)
                .expect("index should always be valid")
                .take()
            {
                Some(entry) => Slot::Frame(entry),
                None => Slot::Missing(seq),
            },
        )
    }

    /// The biggest buffered sequence number.
    ///
    /// Not all slots between `next_released_seq()` and `end_seq()` must be filled.
    #[must_use]
    #[inline]
    pub fn end_seq(&self) -> SequenceNumber {
        self.end
    }

    /// The next sequence number to be popped.
    #[must_use]
    #[inline]
    pub fn next_released_seq(&self) -> SequenceNumber {
        self.next_seq
    }

    /// The currently stored `Entry` count.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The capacity of the `JitterBuf`.
    #[must_use]
    #[inline]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Gets the index for the sequence number.
    #[inline]
    fn slot_index(&self, seq: SequenceNumber) -> Result<usize, OutOfWindow> {
        let Some(offset) = seq.checked_sub(self.next_seq) else {
            return Err(OutOfWindow::Behind);
        };
        if offset >= self.slots.len() as u64 {
            return Err(OutOfWindow::Ahead);
        }
        Ok(self.wrapping_index(seq))
    }

    // When view types are finally stabilized...
    //
    //     /// Get a value from a sequence number.
    //     #[must_use]
    //     fn value_from_seq(&self, seq: SequenceNumber) -> Option<&Entry<T>> {
    //         let index = self.index_from_seq(seq);
    //         self.inner
    //             .get(index)
    //             .expect("index must be in bounds")
    //             .as_ref()
    //     }
    //
    //     /// Get a mutable value from a sequence number.
    //     #[must_use]
    //     fn value_from_seq_mut(&mut self, seq: SequenceNumber) -> &mut Option<Entry<T>> {
    //         let index = self.index_from_seq(seq);
    //         self.inner.get_mut(index).expect("index must be in bounds")
    //     }

    /// Returns the index of the `sequence_number`.
    ///
    /// The index is always valid.
    #[allow(clippy::cast_possible_truncation)] // result < N
    fn wrapping_index(&self, seq: SequenceNumber) -> usize {
        let index = (seq % self.slots.len() as u64) as usize;
        debug_assert!(self.slots.get(index).is_some());

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
    /// Waiting for a frame (startup or after an underrun).
    #[default]
    Idle,
    /// Holding frames. The first slot is released at `start`.
    Buffering { start: Instant },
    /// Releasing one slot per frame duration. The next one is due at `due`.
    Playing { due: Instant },
}

/// A slot released by [`JitterBuf::pop()`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Slot<T> {
    /// The frame arrived in time.
    Frame(Entry<T>),
    /// The frame is lost or too late.
    Missing(SequenceNumber),
}

#[cfg(test)]
mod jitter_buf_test {
    use std::time::{Duration, Instant};

    use audio_common::frame_size::FrameSize;
    use tracing::info;

    use crate::{Entry, JitterBuf, Slot, error::PopError};

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

        for (i, opt_entry) in jb.slots[..N].iter().enumerate() {
            let entry = opt_entry
                .as_ref()
                .expect("entry should have been written to this index");
            assert_eq!(
                entry, &entries[i],
                "Entry {:?} at {} should be {:?}",
                entry, i, entries[i]
            )
        }
    }

    #[test]
    fn test_delay() {
        logging_setup();

        let target_delay = Duration::from_millis(100);
        let mut jb: JitterBuf<i32> = JitterBuf::new(target_delay, FrameSize::default());

        let now = Instant::now();
        let entry = Entry::new(0, now, 32);
        jb.push(entry.clone()).unwrap();

        let (popped, timestamp) = 'wait_loop: loop {
            let now = Instant::now();
            match jb.pop(now) {
                Ok(s) => break 'wait_loop (s, now),
                Err(PopError::Early { due }) => {
                    std::thread::sleep(due.saturating_duration_since(Instant::now()));
                    continue;
                }
                Err(PopError::Empty) => panic!("Can not be empty"),
            };
        };

        match popped {
            Slot::Frame(popped_entry) => assert_eq!(popped_entry, entry),
            Slot::Missing(seq) => panic!("Missing {seq} but must be there"),
        }
        let time_since = timestamp.saturating_duration_since(now);
        let max_delay = jb.delay + jb.frame_size.duration() * 2;
        assert!(
            time_since < max_delay,
            "{} > {}",
            time_since.as_millis(),
            max_delay.as_millis()
        );
    }

    fn logging_setup() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    }
}
