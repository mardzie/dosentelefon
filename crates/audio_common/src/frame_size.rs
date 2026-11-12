use crate::SAMPLE_RATE_48K;

const _: () = assert!(FrameSize::Ms20.samples(SAMPLE_RATE_48K) == 960);

/// Legal Opus frame durations (libopus >= 1.2).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameSize {
    /// CELT-only => no in-band FEC
    Ms2_5 = 1,
    /// CELT-only => no in-band FEC
    Ms5 = 2,
    Ms10 = 4,
    #[default]
    Ms20 = 8,
    Ms40 = 16,
    Ms60 = 24,
    Ms80 = 32,
    Ms100 = 40,
    Ms120 = 48,
}

impl FrameSize {
    const UNIT_US: u64 = 2_500;
    /// 2.5 ms in 1000 ms (1000 / 2.5)
    const UNITS_PER_SECOND: u64 = 1_000_000 / Self::UNIT_US;

    #[must_use]
    #[inline]
    pub const fn samples(self, sample_rate: u32) -> u64 {
        (sample_rate as u64 * self as u64).div_ceil(Self::UNITS_PER_SECOND)
    }

    /// Can support FEC.
    #[must_use]
    pub const fn fec(self) -> bool {
        match self {
            Self::Ms2_5 | Self::Ms5 => false,
            Self::Ms10
            | Self::Ms20
            | Self::Ms40
            | Self::Ms60
            | Self::Ms80
            | Self::Ms100
            | Self::Ms120 => true,
        }
    }

    #[must_use]
    pub const fn application(self) -> opus::Application {
        match self {
            Self::Ms2_5 | Self::Ms5 => opus::Application::LowDelay,
            Self::Ms10
            | Self::Ms20
            | Self::Ms40
            | Self::Ms60
            | Self::Ms80
            | Self::Ms100
            | Self::Ms120 => opus::Application::Voip,
        }
    }

    /// The duration of the `FrameSize`.
    #[must_use]
    pub const fn duration(self) -> std::time::Duration {
        std::time::Duration::from_micros(Self::UNIT_US * self as u64)
    }
}

impl TryFrom<opus::FrameSize> for FrameSize {
    type Error = crate::error::OpusFrameSizeVariantUsupported;

    fn try_from(opus_frame_size: opus::FrameSize) -> Result<Self, Self::Error> {
        use opus::FrameSize as Fs;

        Ok(match opus_frame_size {
            Fs::Ms2_5 => Self::Ms2_5,
            Fs::Ms5 => Self::Ms5,
            Fs::Ms10 => Self::Ms10,
            Fs::Ms20 => Self::Ms20,
            Fs::Ms40 => Self::Ms40,
            Fs::Ms60 => Self::Ms60,
            Fs::Ms80 => Self::Ms80,
            Fs::Ms100 => Self::Ms100,
            Fs::Ms120 => Self::Ms120,
            fs => return Err(crate::error::OpusFrameSizeVariantUsupported(fs)),
        })
    }
}
