use std::sync::{Arc, atomic};

#[derive(Debug)]
pub struct AudioProcessor {
    volume: Arc<atomic::AtomicU8>,

    input_sample_format: cpal::SampleFormat,
}

impl AudioProcessor {
    pub fn new(volume: Arc<atomic::AtomicU8>, input_sample_format: cpal::SampleFormat) -> Self {
        Self {
            volume,

            input_sample_format,
        }
    }

    pub fn process_audio(&self, buf: Vec<f32>) -> Vec<f32> {
        tracing::trace!("Audio Processor: Processing sample {} bytes", buf.len() * 4);

        // Process audio
        // TODO

        buf
    }
}
