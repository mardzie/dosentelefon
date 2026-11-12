use std::fmt::Debug;

use cpal::{
    Device, Host, PauseStreamError, PlayStreamError, Stream, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::audio::{
    config_filter::ConfigFilter,
    error::{DeviceType, Error},
};

pub struct OutputStream {
    device: Device,
    config: SupportedStreamConfig,
    stream: Option<Stream>,
}

impl OutputStream {
    pub fn new(host: &Host) -> Result<Self, Error> {
        let device = host
            .default_output_device()
            .ok_or(Error::DefaultDeviceNotAvailable(DeviceType::Output))?;

        let supported_configs = match device.supported_output_configs() {
            Ok(supported_config) => supported_config,
            Err(e) => {
                return Err(Error::from_supported_stream_configs_error(
                    DeviceType::Output,
                    e,
                ));
            }
        };

        let config = match ConfigFilter::from_supported_output_config(supported_configs)
            .filter_sample_format(cpal::SampleFormat::F32)
            .filter_channel_count_ge(2)
            .get_config_smallest_channel_count()
        {
            Some(config) => config.with_max_sample_rate(),
            None => match device.default_output_config() {
                Ok(default_config) => default_config,
                Err(e) => {
                    return Err(Error::from_default_stream_config_error(
                        DeviceType::Output,
                        e,
                    ));
                }
            },
        };

        tracing::info!("New OutputStream created.");

        Ok(Self {
            device,
            config,
            stream: None,
        })
    }

    pub fn build_stream<T, D, E>(
        &mut self,
        data_callback: D,
        error_callback: E,
    ) -> Result<(), Error>
    where
        T: cpal::SizedSample,
        D: FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        let stream = self
            .device
            .build_output_stream(&self.config.config(), data_callback, error_callback, None)
            .map_err(|e| Error::BuildStream(DeviceType::Output, e))?;

        self.stream = Some(stream);

        Ok(())
    }

    pub fn play(&self) -> Result<(), PlayStreamError> {
        if let Some(stream) = &self.stream {
            stream.play()?;
        };

        Ok(())
    }

    pub fn pause(&self) -> Result<(), PauseStreamError> {
        if let Some(stream) = &self.stream {
            stream.pause()?;
        };

        Ok(())
    }

    pub fn sample_format(&self) -> cpal::SampleFormat {
        self.config.sample_format()
    }

    pub fn config(&self) -> &SupportedStreamConfig {
        &self.config
    }
}

impl Debug for OutputStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputStream")
            .field("device", &self.device.id())
            .field("config", &self.config)
            .finish()
    }
}
