#[derive(thiserror::Error, Debug)]
#[error("payload ({len}) too large (max: {max})")]
pub struct PayloadTooLarge {
    pub len: usize,
    pub max: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum FrameError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PayloadTooLarge(#[from] PayloadTooLarge),
}

#[derive(thiserror::Error, Debug)]
pub enum PostcardFrameError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PayloadTooLarge(#[from] PayloadTooLarge),
    #[error(transparent)]
    Postcard(#[from] postcard::Error),
}
