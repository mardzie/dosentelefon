use std::fmt::Debug;

use cpal::{
    Device, Host, PauseStreamError, PlayStreamError, Stream, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::audio::{
    config_filter::ConfigFilter,
    error::{DeviceType, Error},
};

pub struct InputStream {
    device: Device,
    config: SupportedStreamConfig,
    stream: Option<Stream>,
}

impl InputStream {
    pub fn new(host: &Host) -> Result<Self, Error> {
        let device = host
            .default_input_device()
            .ok_or(Error::DefaultDeviceNotAvailable(DeviceType::Input))?;

        let supported_configs = match device.supported_input_configs() {
            Ok(supported_config) => supported_config,
            Err(e) => {
                return Err(Error::from_supported_stream_configs_error(
                    DeviceType::Input,
                    e,
                ));
            }
        };

        let config = match ConfigFilter::from_supported_input_config(supported_configs)
            .filter_sample_format(cpal::SampleFormat::F32)
            .filter_channel_count_ge(2)
            .get_config_smallest_channel_count()
        {
            Some(config) => config.with_max_sample_rate(),
            None => match device.default_input_config() {
                Ok(default_config) => default_config,
                Err(e) => {
                    return Err(Error::from_default_stream_config_error(
                        DeviceType::Input,
                        e,
                    ));
                }
            },
        };

        tracing::info!("New InputStream created.");

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
        D: FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        self.stream = Some(
            match self.device.build_input_stream(
                &self.config.config(),
                data_callback,
                error_callback,
                None,
            ) {
                Ok(stream) => stream,
                Err(e) => {
                    return Err(Error::BuildStream(DeviceType::Input, e));
                }
            },
        );

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

impl Debug for InputStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputStream")
            .field("device", &self.device.id())
            .field("config", &self.config)
            .finish()
    }
}
