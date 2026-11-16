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

impl TryFrom<PostcardFrameError> for FrameError {
    type Error = PostcardFrameError;

    fn try_from(e: PostcardFrameError) -> Result<Self, Self::Error> {
        match e {
            PostcardFrameError::Io(v) => Ok(Self::Io(v)),
            PostcardFrameError::PayloadTooLarge(v) => Ok(Self::PayloadTooLarge(v)),
            v => Err(v),
        }
    }
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

impl From<FrameError> for PostcardFrameError {
    fn from(e: FrameError) -> Self {
        match e {
            FrameError::Io(v) => Self::Io(v),
            FrameError::PayloadTooLarge(v) => Self::PayloadTooLarge(v),
        }
    }
}
