use audio_common::{SAMPLE_RATE_48K, frame_size::FrameSize};
use opus::Bitrate;

use crate::{DEFAULT_BITRATE, DEFAULT_PACKET_LOSS_PERCENTAGE, error::OpusCodeError};

/// Opus `Encoder`
///
/// One `Encoder` can feed multiple `Decoder`.
#[derive(Debug)]
pub struct Encoder {
    #[allow(clippy::struct_field_names)]
    encoder: opus::Encoder,
    bitrate: u32,
    frame_size: FrameSize,
}

impl Encoder {
    /// Constructs a new `Encoder` with `FRAME_SIZE`.
    pub fn new() -> Result<Self, opus::Error> {
        let frame_size = FrameSize::default();
        Self::from_frame_size(frame_size)
    }

    /// Constructs a new `Encoder`.
    ///
    /// `frame_size` must be the same as in the `Decoder`.
    pub fn from_frame_size(frame_size: FrameSize) -> Result<Self, opus::Error> {
        let mut encoder = opus::Encoder::new(
            SAMPLE_RATE_48K,
            opus::Channels::Mono,
            frame_size.application(),
        )?;
        // Variable bitrate must be `false`.
        // VBR leaks encrypted speech through packet sizes.
        // DTX must be off too or it can leak information through encryption.
        encoder.set_vbr(false)?;
        encoder.set_dtx(false)?;
        encoder.set_inband_fec(frame_size.fec())?;
        encoder.set_packet_loss_perc(i32::from(DEFAULT_PACKET_LOSS_PERCENTAGE))?;

        let mut this = Self {
            encoder,
            bitrate: DEFAULT_BITRATE,
            frame_size,
        };
        this.set_bitrate(DEFAULT_BITRATE)?;

        Ok(this)
    }

    /// Encode an Opus frame from floating point input.
    #[inline]
    pub fn encode(&mut self, samples: &[f32], packet: &mut [u8]) -> Result<usize, OpusCodeError> {
        if samples.len() != self.input_sample_len() {
            return Err(OpusCodeError::InvalidInputSize {
                current: samples.len(),
                needed: self.input_sample_len(),
            });
        }
        Ok(self.encoder.encode_float(samples, packet)?)
    }

    /// Get the per channel input sample count.
    #[must_use]
    #[inline]
    pub fn input_sample_len(&self) -> usize {
        usize::try_from(self.frame_size.samples(SAMPLE_RATE_48K))
            .expect("target should support u64 to usize conversions")
    }

    /// Forward Error Correction requested.
    #[must_use]
    #[inline]
    pub fn fec(&self) -> bool {
        self.frame_size.fec()
    }

    /// Sets the encoder’s expected packet loss percentage.
    ///
    /// Clamps the percentage to a valid 0 to 100 range.
    #[inline]
    pub fn set_packet_loss_percentage(&mut self, percentage: u8) -> Result<(), opus::Error> {
        let clamped_percentage = percentage.clamp(0, 100);
        tracing::debug!("Setting opus encoder packet loss percentage to {clamped_percentage} %");
        self.encoder
            .set_packet_loss_perc(i32::from(clamped_percentage))
    }

    /// Set the encoder's bitrate.
    ///
    /// Clamps the bitrate `500` to `300000`.
    #[inline]
    pub fn set_bitrate(&mut self, bitrate: u32) -> Result<(), opus::Error> {
        // Min/max come from `OPUS_SET_BITRATE_REQUEST` in
        // https://gitlab.xiph.org/xiph/opus/-/raw/1632152b83b8ab4e28393bca94450796b71b0201/src/opus_encoder.c
        let clamped_bitrate = bitrate.clamp(500, 300_000);
        self.encoder
            .set_bitrate(Bitrate::Bits(clamped_bitrate as i32))?;
        self.bitrate = clamped_bitrate;
        tracing::debug!("Set opus encoder bitrate to {}", clamped_bitrate);
        Ok(())
    }

    #[must_use]
    #[inline]
    pub fn bitrate(&self) -> u32 {
        self.bitrate
    }

    #[inline]
    pub fn reset_state(&mut self) -> Result<(), opus::Error> {
        tracing::debug!("Resetting opus encoder state");
        self.encoder.reset_state()
    }
}
