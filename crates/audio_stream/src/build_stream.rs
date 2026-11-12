macro_rules! build_stream {
    (
        $device:expr,
        $build:ident,
        $config:expr,
        $callback:path,
        $args:tt,
        { $($variant:ident),+ $(,)? }
    ) => {{
        let device: &::cpal::Device = $device;
        let config: &::cpal::SupportedStreamConfig = &$config;
        match config.sample_format() {
            $(
            ::cpal::SampleFormat::$variant => ::cpal::traits::DeviceTrait::$build::<
                $crate::build_stream::cpal_sample_format_type!($variant),
                _,
                _,
            >(
                device,
                config.config(),
                move |buf, info| {
                    $crate::build_stream::call_with!($callback, buf, info, $args)
                },
                // TODO: names the `SampleType` instead of the direction.
                |e| ::tracing::error!("{} stream error: {}", stringify!($variant), e),
                None,
            )
            .map_err($crate::error::StreamBuildError::from),
            )+
            format => Err($crate::error::StreamBuildError::UnsupportedSampleFormat(format))
        }
    }};
}
pub(crate) use build_stream;

macro_rules! call_with {
    ($callback:path, $buf:expr, $info:expr, ($($arg:expr),* $(,)?)) => {
        $callback($buf, $info, $($arg),*)
    };
}
pub(crate) use call_with;

macro_rules! cpal_sample_format_type {
    (F32) => {
        f32
    };
    (F64) => {
        f64
    };
    (U8) => {
        u8
    };
    (U16) => {
        u16
    };
    (U24) => {
        ::cpal::U24
    };
    (U32) => {
        u32
    };
    (U64) => {
        u64
    };
    (I8) => {
        i8
    };
    (I16) => {
        i16
    };
    (I24) => {
        ::cpal::I24
    };
    (I32) => {
        i32
    };
    (I64) => {
        i64
    };
}
pub(crate) use cpal_sample_format_type;
