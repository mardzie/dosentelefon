use audio_common::{SAMPLE_RATE_48K, frame_size::FrameSize};
use audio_stream::{Direction, Stream};

use crate::inner::ResamplerInner;

mod inner;

/// `Resampler`
///
/// The `Resampler` is stateful.
/// One `Resampler` per stream.
#[derive(Debug)]
pub struct Resampler {
    inner: ResamplerInner,
    frame_size: FrameSize,
}

impl Resampler {
    #[must_use]
    pub fn new<D>(stream: &Stream<D>) -> Result<Self, rubato::ResamplerConstructionError>
    where
        D: Direction,
    {
        Self::from_frame_size(stream, FrameSize::default())
    }

    #[must_use]
    pub fn from_frame_size<D>(
        stream: &Stream<D>,
        frame_size: FrameSize,
    ) -> Result<Self, rubato::ResamplerConstructionError>
    where
        D: Direction,
    {
        let inner = ResamplerInner::new(stream, frame_size.samples(SAMPLE_RATE_48K) as u32)?;
        Ok(Self { inner, frame_size })
    }

    /// The maximum input buffer size.
    #[must_use]
    #[inline]
    pub fn max_input(&self) -> usize {
        self.inner.max_input(self.frame_at_48k())
    }

    /// The maximum output buffer size.
    #[must_use]
    #[inline]
    pub fn max_output(&self) -> usize {
        self.inner.max_output(self.frame_at_48k())
    }

    /// Get the number of input frames per channel for the next call to `resample()`.
    #[must_use]
    #[inline]
    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next(self.frame_at_48k())
    }

    /// Get the number of output frames per channel for the next call to `resample()`
    #[must_use]
    #[inline]
    pub fn output_frames_next(&self) -> usize {
        self.inner.output_frames_next(self.frame_at_48k())
    }

    /// One side is always a fixed length.
    ///
    /// `DeviceType::Input` -> Output is a fixed size, Input dynamic.
    ///
    /// `DeviceType::Output` -> Input is a fixed size, Output dynamic.
    ///
    /// The dynamic side has to be queried with `input_frames_next()` or `output_frames_next()`.
    /// The slices may be greater than the requested size but only the requested frames will be processed.
    ///
    /// Returns the number of frames consumed and produced as a tuple `(consumed, produced)`.
    pub fn resample(
        &mut self,
        input_buf: &[f32],
        output_buf: &mut [f32],
    ) -> Result<(usize, usize), rubato::ResampleError> {
        self.inner
            .resample(input_buf, output_buf, self.frame_at_48k())
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[must_use]
    fn frame_at_48k(&self) -> usize {
        usize::try_from(self.frame_size.samples(SAMPLE_RATE_48K))
            .expect("can not convert u64 to usize")
    }
}
