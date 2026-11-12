#![warn(clippy::pedantic)]

use std::num::NonZeroUsize;

pub mod error;
pub mod frame_size;

pub const SAMPLE_RATE_48K: u32 = 48_000;
pub const CHANNELS: NonZeroUsize = NonZeroUsize::new(1).unwrap();
