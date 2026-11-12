use audio_common::{SAMPLE_RATE_48K, frame_size::FrameSize};

use crate::error::OpusCodeError;

/// Opus `Decoder`
///
/// The `Decoder` is stateful. One `Decoder` per stream.
#[derive(Debug)]
pub struct Decoder {
    #[allow(clippy::struct_field_names)]
    decoder: opus::Decoder,
    frame_size: FrameSize,
}

impl Decoder {
    /// Constructs a new `Decoder` with `FRAME_SIZE`.
    pub fn new() -> Result<Self, opus::Error> {
        let frame_size = FrameSize::default();
        Self::from_frame_size(frame_size)
    }

    /// Constructs a new `Decoder`.
    ///
    /// `frame_size` must be the same as in the `Encoder`.
    pub fn from_frame_size(frame_size: FrameSize) -> Result<Decoder, opus::Error> {
        Ok(Self {
            decoder: opus::Decoder::new(SAMPLE_RATE_48K, opus::Channels::Mono)?,
            frame_size,
        })
    }

    /// Decode an Opus packet with floating point output.
    ///
    /// To represent packet loss, pass `DecodeInput::Plc`.
    ///
    /// The return value is the number of samples per channel decoded from the packet.
    #[inline]
    pub fn decode(
        &mut self,
        packet: DecodeInput,
        samples: &mut [f32],
    ) -> Result<usize, OpusCodeError> {
        if samples.len() < self.output_sample_len() {
            return Err(OpusCodeError::InvalidOutputSize {
                current: samples.len(),
                needed: self.output_sample_len(),
            });
        }

        let (packet, fec): (&[u8], bool) = match packet {
            DecodeInput::Packet(slice) => {
                if slice.is_empty() {
                    return Err(OpusCodeError::EmptyInput);
                }
                (slice, false)
            }
            DecodeInput::Fec { next } => {
                if next.is_empty() {
                    return Err(OpusCodeError::EmptyInput);
                }
                (next, true)
            }
            DecodeInput::Plc => (&[] as &[u8], false),
        };
        Ok(self.decoder.decode_float(packet, samples, fec)?)
    }

    /// Get the per channel output sample count.
    #[must_use]
    #[inline]
    pub fn output_sample_len(&self) -> usize {
        usize::try_from(self.frame_size.samples(SAMPLE_RATE_48K))
            .expect("target should support u64 to usize conversions")
    }

    #[inline]
    pub fn reset_state(&mut self) -> Result<(), opus::Error> {
        self.decoder.reset_state()
    }
}

/// Input for one playout slot.
#[must_use]
#[derive(Debug)]
pub enum DecodeInput<'a> {
    /// The slot's own packet arrived.
    ///
    /// -> `decode_float(pkt, &mut out[..FRAME], false)`
    Packet(&'a [u8]),

    /// The slot is lost but the next packet arrived: decode the copy embedded in it.
    /// (Don't forget to call again with `DecodeInput::Packet(next)` or else next isn't being decoded.)
    ///
    /// -> `decode_float(next, &mut out[..FRAME], true)`
    ///
    /// (FEC = Forward Error Correction)
    Fec { next: &'a [u8] },

    /// The slot and the next packet are both missing.
    ///
    /// -> `decode_float(&[], &mut out[..FRAME], false)`
    ///
    /// (PLC = Packet Loss Concealment)
    Plc,
}
