use std::fmt::Display;

use cpal::{DefaultStreamConfigError, SupportedStreamConfigsError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("default {0} device not available.")]
    DefaultDeviceNotAvailable(DeviceType),
    #[error("{0} device not available")]
    DeviceNotAvailable(DeviceType),
    #[error("{0} backend specific error: {1}")]
    BackendSpecific(DeviceType, cpal::BackendSpecificError),
    #[error("failed to build {0} stream: {1}")]
    BuildStream(DeviceType, cpal::BuildStreamError),
    #[error("ivalid argument in {0} Device")]
    InvalidArgument(DeviceType),
}

#[derive(Debug)]
pub enum DeviceType {
    Input,
    Output,
}

impl Error {
    pub fn from_supported_stream_configs_error(
        device_type: DeviceType,
        e: SupportedStreamConfigsError,
    ) -> Self {
        match e {
            SupportedStreamConfigsError::DeviceNotAvailable => {
                Error::DeviceNotAvailable(device_type)
            }
            SupportedStreamConfigsError::InvalidArgument => Error::InvalidArgument(device_type),
            SupportedStreamConfigsError::BackendSpecific { err } => {
                Error::BackendSpecific(device_type, err)
            }
        }
    }

    pub fn from_default_stream_config_error(
        device_type: DeviceType,
        e: DefaultStreamConfigError,
    ) -> Self {
        match e {
            DefaultStreamConfigError::DeviceNotAvailable => {
                Error::DefaultDeviceNotAvailable(device_type)
            }
            DefaultStreamConfigError::StreamTypeNotSupported => {
                panic!("Default input stream config not supported.")
            }
            DefaultStreamConfigError::BackendSpecific { err } => {
                Error::BackendSpecific(device_type, err)
            }
        }
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "Input"),
            Self::Output => write!(f, "Output"),
        }
    }
}
