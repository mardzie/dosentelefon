use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    num::NonZero,
};

use audio_common::SAMPLE_RATE_48K;
use cpal::{
    ChannelCount, Device, SampleFormat, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use ringbuf::{HeapCons, HeapProd, HeapRb, traits::Split};

use build_stream::build_stream;

use crate::error::StreamBuildError;

mod build_stream;
mod sealed {
    use cpal::{Device, SupportedStreamConfig};
    use ringbuf::HeapRb;

    use crate::{DeviceType, error::StreamBuildError};

    pub trait SealedDirection {
        const DEVICE_TYPE: DeviceType;

        type SupportedConfigs: Iterator<Item = cpal::SupportedStreamConfigRange>;
        /// Ring half returned to the caller: `HeapCons<f32>` for Input, `HeapProd<f32>` for Output.
        type Handle;

        fn default_device(host: &cpal::Host) -> Option<Device>;
        fn supported_configs(device: &Device) -> Result<Self::SupportedConfigs, cpal::Error>;
        fn default_config(device: &Device) -> Result<cpal::SupportedStreamConfig, cpal::Error>;
        fn build(
            device: &Device,
            config: SupportedStreamConfig,
            rb: HeapRb<f32>,
        ) -> Result<(cpal::Stream, Self::Handle), StreamBuildError>;
    }
}

pub mod error;

/// The Direction of a [`Stream<D>`].
pub trait Direction: sealed::SealedDirection + 'static {}

/// Input device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input;
impl Direction for Input {}
impl sealed::SealedDirection for Input {
    const DEVICE_TYPE: DeviceType = DeviceType::Input;

    type SupportedConfigs = cpal::SupportedInputConfigs;
    type Handle = HeapCons<f32>;

    fn default_device(host: &cpal::Host) -> Option<Device> {
        host.default_input_device()
    }

    fn supported_configs(device: &Device) -> Result<Self::SupportedConfigs, cpal::Error> {
        device.supported_input_configs()
    }

    fn default_config(device: &Device) -> Result<cpal::SupportedStreamConfig, cpal::Error> {
        device.default_input_config()
    }

    fn build(
        device: &Device,
        config: SupportedStreamConfig,
        rb: HeapRb<f32>,
    ) -> Result<(cpal::Stream, Self::Handle), StreamBuildError> {
        let (mut producer, consumer) = rb.split();
        let channels = non_zero_channels(&config);
        let stream = build_stream!(
            device,
            build_input_stream,
            config,
            audio_callback::input_audio_callback,
            (&mut producer, channels),
            {
                F32,
                F64,
                U8,
                U16,
                U24,
                U32,
                U64,
                I8,
                I16,
                I24,
                I32,
                I64,
            }
        )?;
        Ok((stream, consumer))
    }
}

/// Output device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Output;
impl Direction for Output {}
impl sealed::SealedDirection for Output {
    const DEVICE_TYPE: DeviceType = DeviceType::Output;

    type SupportedConfigs = cpal::SupportedOutputConfigs;
    type Handle = HeapProd<f32>;

    fn default_device(host: &cpal::Host) -> Option<Device> {
        host.default_output_device()
    }

    fn supported_configs(device: &Device) -> Result<Self::SupportedConfigs, cpal::Error> {
        device.supported_output_configs()
    }

    fn default_config(device: &Device) -> Result<cpal::SupportedStreamConfig, cpal::Error> {
        device.default_output_config()
    }

    fn build(
        device: &Device,
        config: SupportedStreamConfig,
        rb: HeapRb<f32>,
    ) -> Result<(cpal::Stream, Self::Handle), StreamBuildError> {
        let (producer, mut consumer) = rb.split();
        let channels = non_zero_channels(&config);
        let stream = build_stream!(
            device,
            build_output_stream,
            config,
            audio_callback::output_audio_callback,
            (&mut consumer, channels),
            {
                F32,
                F64,
                U8,
                U16,
                U24,
                U32,
                U64,
                I8,
                I16,
                I24,
                I32,
                I64,
            }
        )?;
        Ok((stream, producer))
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Input,
    Output,
}

impl From<Input> for DeviceType {
    fn from(_: Input) -> Self {
        Self::Input
    }
}

impl From<Output> for DeviceType {
    fn from(_: Output) -> Self {
        Self::Output
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Input => write!(f, "Input"),
            DeviceType::Output => write!(f, "Output"),
        }
    }
}

/// A cpal Stream wrapper.
///
/// Works with mono `f32` (-1.0 <= s < 1.0) audio samples.
///
/// The Stream will play as soon as it is constructed.
///
/// # [`Input`]
/// A `Stream<Input>` collects samples from the input device into the ring.
/// The ring needs to be emptied regularly.
/// If the ring is at capacity, new samples from the output device will be dropped.
/// Only the freshest samples will be written into the ring if the rings free space is insufficient.
/// The devices [`SampleFormat`] will be converted to `f32`.
/// All channels will be averaged to mono audio.
///
/// # [`Output`]
/// A `Stream<Output>` writes samples from the ring into the output devices buffer.
/// The ring needs to be filled regularly.
/// If there aren't enough samples in the ring to satisfy the output devices buffer, the audio wont behave as expected.
/// The `f32` samples from the ring will be converted to the devices [`SampleFormat`].
/// The ring samples will be written to all channels.
pub struct Stream<D> {
    device: Device,
    config: SupportedStreamConfig,
    /// Held for its `Drop`. Dropping stops the callback.
    #[allow(clippy::struct_field_names)]
    stream: cpal::Stream,
    _direction: PhantomData<D>,
}

impl<D: Direction> Stream<D> {
    /// New default device from `host` and a ring buffer capacity of `capacity`.
    ///
    /// # Errors
    ///
    /// Returns `Err(StreamBuildError::DefaultDeviceUnavailable)`
    pub fn new(host: &cpal::Host, capacity: usize) -> Result<(Self, D::Handle), StreamBuildError> {
        let device = D::default_device(host)
            .ok_or(StreamBuildError::DefaultDeviceUnavailable(D::DEVICE_TYPE))?;
        Self::from_device(device, capacity)
    }

    /// From a device with a ring buffer capacity of `capacity`.
    ///
    /// # Errors
    ///
    /// Returns `Err(StreamBuildError::DefaultDeviceUnavailable)`
    pub fn from_device(
        device: Device,
        capacity: usize,
    ) -> Result<(Self, D::Handle), StreamBuildError> {
        let config = pick_config::<D>(&device)?;
        tracing::debug!("{} Stream config: {:?}", D::DEVICE_TYPE, config);

        let (stream, handle) = D::build(&device, config, HeapRb::new(capacity))?;
        let this = Self {
            device,
            config,
            stream,
            _direction: PhantomData,
        };
        this.stream.play()?;

        Ok((this, handle))
    }

    /// The configuration of the underlying cpal stream.
    #[must_use]
    pub fn config(&self) -> SupportedStreamConfig {
        self.config
    }

    /// The [`DeviceType`].
    pub fn device_type(&self) -> DeviceType {
        D::DEVICE_TYPE
    }
}

impl<D: Direction> Debug for Stream<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("device", &self.device)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

fn pick_config<D: Direction>(device: &Device) -> Result<SupportedStreamConfig, cpal::Error> {
    let config = match preferred_config_filter(D::supported_configs(device)?) {
        Some(config) => config,
        None => D::default_config(device)?,
    };
    Ok(config)
}

/// Ranks the `format`.
///
/// Must be kept in sync with `cpal_sample_format_type!`.
#[must_use]
fn format_rank(format: SampleFormat) -> Option<u8> {
    use SampleFormat as F;
    Some(match format {
        F::F32 => 0,
        F::F64 => 1,
        F::I32 => 2,
        F::U32 => 3,
        F::I16 => 4,
        F::U16 => 5,
        F::I24 => 6,
        F::U24 => 7,
        F::I64 => 8,
        F::U64 => 9,
        F::I8 => 10,
        F::U8 => 11,
        _ => return None,
    })
}

#[must_use]
fn preferred_config_filter(
    config_iter: impl IntoIterator<Item = cpal::SupportedStreamConfigRange>,
) -> Option<cpal::SupportedStreamConfig> {
    config_iter
        .into_iter()
        .filter_map(|r| r.try_with_sample_rate(SAMPLE_RATE_48K))
        .filter_map(|c| format_rank(c.sample_format()).map(|rank| (rank, c)))
        .min_by_key(|(rank, c)| (*rank, c.channels()))
        .map(|(_, c)| c)
}

fn non_zero_channels(config: &cpal::SupportedStreamConfig) -> NonZero<ChannelCount> {
    NonZero::new(config.channels()).expect("CPAL reported an invalid zero-channel configuration")
}

/// Clamps a `f32` sample to `-1.0 <= sample < 1.0`.
#[must_use = "function returns a new number and does not mutate the original value"]
#[inline]
pub const fn clamp_f32_sample(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0 - f32::EPSILON / 2.0)
}

mod audio_callback {
    use std::num::NonZero;

    use cpal::{ChannelCount, Sample};
    use ringbuf::{
        HeapCons, HeapProd,
        traits::{Consumer, Observer, Producer},
    };

    use crate::clamp_f32_sample;

    pub fn input_audio_callback<T>(
        buf: &[T],
        _: &cpal::InputCallbackInfo,
        producer: &mut HeapProd<f32>,
        channels: NonZero<ChannelCount>,
    ) where
        T: cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        // cpal guarantees that each frame has all channels.
        debug_assert_eq!(buf.len() % channels.get() as usize, 0);

        let channels = channels.get() as usize;
        let inverse_channels = 1.0 / channels as f32;

        let mono_len = buf.len() / channels;
        let vacant_len = producer.vacant_len();
        // Drop the oldest samples.
        let dropped = mono_len.saturating_sub(vacant_len);
        let buf_start = dropped * channels;
        let mono = buf[buf_start..].chunks_exact(channels).map(|frame| {
            // Average all channels into one mono frame per chunk.
            frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() * inverse_channels
        });
        let pushed = producer.push_iter(mono);
        let overrun = mono_len.saturating_sub(pushed);

        // The overrun are samples that could not be written to the producer.
        // The space of the producer should never decrease without writing to it.
        // The oldest samples that dont fit get dropped first.
        // If then the overrun isnt the same as the dropped then the producer had not as much space as advertised.
        debug_assert_eq!(overrun, dropped);
    }

    pub fn output_audio_callback<T>(
        // Buffer already comes silenced from cpal >= 0.18. Just write in it.
        buf: &mut [T],
        _: &cpal::OutputCallbackInfo,
        consumer: &mut HeapCons<f32>,
        channels: NonZero<ChannelCount>,
    ) where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        // cpal guarantees that each frame has all channels.
        debug_assert_eq!(buf.len() % channels.get() as usize, 0);

        for (frame, sample) in buf
            .chunks_exact_mut(channels.get() as usize)
            .zip(consumer.pop_iter().map(clamp_f32_sample))
        {
            frame.fill(sample.to_sample());
        }
    }
}
