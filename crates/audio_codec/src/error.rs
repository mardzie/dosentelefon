#[derive(thiserror::Error, Debug)]
pub enum OpusCodeError {
    #[error(transparent)]
    Opus(#[from] opus::Error),
    #[error("invalid input size: len {current}, needed {needed}")]
    InvalidInputSize { current: usize, needed: usize },
    #[error("invalid output size: len {current}, needed {needed}")]
    InvalidOutputSize { current: usize, needed: usize },
    #[error("empty input")]
    EmptyInput,
}
