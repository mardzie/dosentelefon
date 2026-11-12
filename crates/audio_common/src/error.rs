#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("opus frame size variant unsupported")]
pub struct OpusFrameSizeVariantUsupported(pub opus::FrameSize);
