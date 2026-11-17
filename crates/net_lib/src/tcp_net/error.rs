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
    #[error("poisoned")]
    Poisoned,
    #[error("closed")]
    Closed,
    #[error("unexpected EOF")]
    UnexpectedEOF,
}

impl IsRetryable for FrameError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            FrameError::Io(e) if is_retryable(&e)
        )
    }
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
    #[error("poisoned")]
    Poisoned,
    #[error("closed")]
    Closed,
    #[error("unexpected EOF")]
    UnexpectedEOF,
    #[error(transparent)]
    Postcard(#[from] postcard::Error),
}

impl IsRetryable for PostcardFrameError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            PostcardFrameError::Io(e) if is_retryable(&e)
        )
    }
}

impl From<FrameError> for PostcardFrameError {
    fn from(e: FrameError) -> Self {
        match e {
            FrameError::Io(v) => Self::Io(v),
            FrameError::PayloadTooLarge(v) => Self::PayloadTooLarge(v),
            FrameError::Poisoned => Self::Poisoned,
            FrameError::Closed => Self::Closed,
            FrameError::UnexpectedEOF => Self::Closed,
        }
    }
}

fn is_retryable(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;

    matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}
