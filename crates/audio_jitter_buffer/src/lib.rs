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
                Some(entry) => {
                    self.len -= 1;
                    Slot::Frame(entry)
                }
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

    use crate::{Entry, JitterBuf, SequenceNumber, Slot, error::PopError};

    const DELAY: Duration = Duration::from_millis(100);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Out {
        Frame(SequenceNumber),
        Missing(SequenceNumber),
    }

    fn setup<T>() -> (JitterBuf<T>, Instant, Duration) {
        let fs = FrameSize::default();
        let frame = fs.duration();
        (JitterBuf::new(DELAY, fs), Instant::now(), frame)
    }

    fn drain<T>(jb: &mut JitterBuf<T>, now: Instant) -> Vec<Out> {
        let mut out = Vec::new();
        while let Ok(slot) = jb.pop(now) {
            out.push(match slot {
                Slot::Frame(e) => Out::Frame(e.seq()),
                Slot::Missing(seq) => Out::Missing(seq),
            });
            check_invariants(jb);
        }
        out
    }

    fn push(jb: &mut JitterBuf<SequenceNumber>, seq: SequenceNumber, at: Instant) {
        jb.push(Entry::new(seq, at, seq)).unwrap();
        check_invariants(jb);
    }

    fn check_invariants<T>(jb: &JitterBuf<T>) {
        assert!(jb.next_seq <= jb.end);
        assert!(jb.end <= jb.next_seq + jb.capacity() as u64);
        assert_eq!(jb.len, jb.slots.iter().flatten().count());
    }

    #[test]
    fn release_exactly_at_delay() {
        let (mut jb, t0, _) = setup();
        push(&mut jb, 0, t0);
        let due = t0 + DELAY;
        assert!(matches!(jb.pop(t0), Err(PopError::Early { due: d }) if d == due));
        assert!(matches!(
            jb.pop(due - Duration::from_nanos(1)),
            Err(PopError::Early { .. })
        ));
        assert_eq!(drain(&mut jb, due), [Out::Frame(0)])
    }

    #[test]
    fn gap_during_playback_yields_missing() {
        let (mut jb, t0, frame) = setup();
        push(&mut jb, 0, t0);
        push(&mut jb, 2, t0 + 2 * frame);
        assert_eq!(
            drain(&mut jb, t0 + DELAY + 2 * frame),
            [Out::Frame(0), Out::Missing(1), Out::Frame(2)]
        )
    }

    #[test]
    fn underrun_rebuffers_with_full_delay() {
        let (mut jb, t0, frame) = setup();
        push(&mut jb, 0, t0);
        assert_eq!(drain(&mut jb, t0 + DELAY + frame), [Out::Frame(0)]); // Second pop underruns
        let t1 = t0 + 10 * frame;
        push(&mut jb, 1, t1);
        assert!(matches!(jb.pop(t1), Err(PopError::Early { due: d }) if d == t1 + DELAY))
    }

    #[test]
    fn ordering() {
        let (mut jb, t0, frame) = setup();
        push(&mut jb, 2, t0);
        push(&mut jb, 0, t0);
        push(&mut jb, 1, t0);
        assert_eq!(
            drain(&mut jb, t0 + DELAY + 2 * frame),
            [Out::Frame(0), Out::Frame(1), Out::Frame(2)]
        );
    }
}
