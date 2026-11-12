use std::ops::{Deref, DerefMut};

use cpal::{ChannelCount, SupportedStreamConfigRange};
pub use cpal::{SampleFormat, SupportedInputConfigs, SupportedOutputConfigs};

#[derive(Debug, Clone)]
pub struct ConfigFilter {
    config: Vec<SupportedStreamConfigRange>,
}

impl ConfigFilter {
    pub fn from_supported_input_config(config: SupportedInputConfigs) -> Self {
        Self {
            config: config.collect(),
        }
    }

    pub fn from_supported_output_config(config: SupportedOutputConfigs) -> Self {
        Self {
            config: config.collect(),
        }
    }

    /// Filter for only Configs that use the `sample_format`.
    pub fn filter_sample_format(self, sample_format: SampleFormat) -> Self {
        let filtered = self
            .config
            .into_iter()
            .filter(|x| x.sample_format() == sample_format)
            .collect();

        Self { config: filtered }
    }

    /// Filter for only Configs with a channel count equals to `channel_count`.
    pub fn filter_channel_count_eq(self, channel_count: ChannelCount) -> Self {
        let filtered = self
            .config
            .into_iter()
            .filter(|x| x.channels() == channel_count)
            .collect();

        Self { config: filtered }
    }

    pub fn filter_channel_count_ge(self, channel_count: ChannelCount) -> Self {
        let filetered = self
            .config
            .into_iter()
            .filter(|x| x.channels() >= channel_count)
            .collect();

        Self { config: filetered }
    }

    pub fn filter_channel_count_le(self, channel_count: ChannelCount) -> Self {
        let filtered = self
            .config
            .into_iter()
            .filter(|x| x.channels() < channel_count)
            .collect();

        Self { config: filtered }
    }

    pub fn get_config_smallest_channel_count(self) -> Option<SupportedStreamConfigRange> {
        self.config.into_iter().reduce(|x, acc| {
            if x.channels() < acc.channels() {
                x
            } else {
                acc
            }
        })
    }

    pub fn pop(&mut self) -> Option<SupportedStreamConfigRange> {
        self.config.pop()
    }

    pub fn inner(self) -> Vec<SupportedStreamConfigRange> {
        self.config
    }
}

impl Deref for ConfigFilter {
    type Target = Vec<SupportedStreamConfigRange>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl DerefMut for ConfigFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}
