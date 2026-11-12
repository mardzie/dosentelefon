use audio_common::{CHANNELS, SAMPLE_RATE_48K};
use audio_stream::{DeviceType, Direction, Stream};
use rubato::{Fft, Resampler, audioadapter_buffers::direct::SequentialSlice};

#[derive(Debug)]
pub enum ResamplerInner {
    Native,
    Convert {
        resampler: Fft<f32>,
        device_type: DeviceType,
    },
}

impl ResamplerInner {
    #[must_use]
    pub fn new<D>(
        stream: &Stream<D>,
        chunk_size: u32,
    ) -> Result<Self, rubato::ResamplerConstructionError>
    where
        D: Direction,
    {
        let sample_rate_input: usize;
        let sample_rate_output: usize;
        match stream.device_type() {
            DeviceType::Input => {
                sample_rate_input = stream.config().sample_rate() as usize;
                sample_rate_output = SAMPLE_RATE_48K as usize;
            }
            DeviceType::Output => {
                sample_rate_input = SAMPLE_RATE_48K as usize;
                sample_rate_output = stream.config().sample_rate() as usize;
            }
        }

        Self::from_sample_rates(
            stream.device_type(),
            sample_rate_input,
            sample_rate_output,
            chunk_size,
        )
    }

    #[must_use]
    pub fn from_sample_rates(
        device_type: DeviceType,
        sample_rate_input: usize,
        sample_rate_output: usize,
        chunk_size: u32,
    ) -> Result<Self, rubato::ResamplerConstructionError> {
        if sample_rate_input == sample_rate_output {
            Ok(Self::Native)
        } else {
            const _: () = assert!(
                usize::BITS >= u32::BITS,
                "only 32 bit and up targets are supported"
            );
            Ok(Self::Convert {
                resampler: rubato::Fft::<f32>::new(
                    sample_rate_input,
                    sample_rate_output,
                    chunk_size as usize,
                    CHANNELS.get(),
                    device_type.direction(),
                )?,
                device_type,
            })
        }
    }

    /// The maximum input buffer size.
    #[must_use]
    pub fn max_input(&self, frame: usize) -> usize {
        match self {
            ResamplerInner::Native => frame,
            ResamplerInner::Convert { resampler, .. } => resampler.input_frames_max(),
        }
    }

    /// The maximum output buffer size.
    #[must_use]
    pub fn max_output(&self, frame: usize) -> usize {
        match self {
            ResamplerInner::Native => frame,
            ResamplerInner::Convert { resampler, .. } => resampler.output_frames_max(),
        }
    }

    /// Get the number of input frames per channel for the next call to `resample()`.
    #[must_use]
    pub fn input_frames_next(&self, frame: usize) -> usize {
        match self {
            ResamplerInner::Native => frame,
            ResamplerInner::Convert { resampler, .. } => Self::input_frames_next_inner(resampler),
        }
    }

    #[must_use]
    fn input_frames_next_inner(resampler: &Fft<f32>) -> usize {
        resampler.input_frames_next()
    }

    /// Get the number of output frames per channel for the next call to `resample()`
    #[must_use]
    pub fn output_frames_next(&self, frame: usize) -> usize {
        match self {
            ResamplerInner::Native => frame,
            ResamplerInner::Convert { resampler, .. } => Self::output_frames_next_inner(resampler),
        }
    }

    #[must_use]
    fn output_frames_next_inner(resampler: &Fft<f32>) -> usize {
        resampler.output_frames_next()
    }

    pub fn resample(
        &mut self,
        input_buf: &[f32],
        output_buf: &mut [f32],
        frames: usize,
    ) -> Result<(usize, usize), rubato::ResampleError> {
        match self {
            ResamplerInner::Native => {
                debug_assert!(
                    input_buf.len() >= frames,
                    "Input buffer needs to be at least {frames} long",
                );
                debug_assert!(
                    output_buf.len() >= frames,
                    "Output buffer needs to be at least {frames} long",
                );

                output_buf[..frames].copy_from_slice(&input_buf[..frames]);
                Ok((frames, frames))
            }
            ResamplerInner::Convert {
                resampler,
                device_type,
            } => {
                debug_assert!(
                    input_buf.len() >= Self::input_frames_next_inner(resampler),
                    "Input buffer needs to be at least {} long",
                    Self::input_frames_next_inner(resampler)
                );
                debug_assert!(
                    output_buf.len() >= Self::output_frames_next_inner(resampler),
                    "Output buffer needs to be at least {} long",
                    Self::output_frames_next_inner(resampler)
                );

                let (input_adapter, mut output_adapter) =
                    Self::slice_to_sequential(input_buf, output_buf, device_type.clone(), frames);
                let (consumed, produced) =
                    resampler.process_into_buffer(&input_adapter, &mut output_adapter, None)?;
                Ok((consumed, produced))
            }
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Native => (),
            Self::Convert { resampler, .. } => {
                resampler.reset();
            }
        }
    }

    #[must_use]
    fn slice_to_sequential<'input, 'output>(
        input_buf: &'input [f32],
        output_buf: &'output mut [f32],
        device_type: DeviceType,
        frames: usize,
    ) -> (
        SequentialSlice<&'input [f32]>,
        SequentialSlice<&'output mut [f32]>,
    ) {
        let (input_adapter_frames, output_adapter_frames) = match device_type {
            DeviceType::Input => (input_buf.len(), frames),
            DeviceType::Output => (frames, output_buf.len()),
        };

        (
            SequentialSlice::new(input_buf, CHANNELS.get(), input_adapter_frames)
                .expect("input adapter size mismatch"),
            SequentialSlice::new_mut(output_buf, CHANNELS.get(), output_adapter_frames)
                .expect("output adapter size mismatch"),
        )
    }
}

trait ResampleDirection {
    fn direction(self) -> rubato::FixedSync;
}
impl ResampleDirection for DeviceType {
    fn direction(self) -> rubato::FixedSync {
        match self {
            Self::Input => rubato::FixedSync::Output,
            Self::Output => rubato::FixedSync::Input,
        }
    }
}
