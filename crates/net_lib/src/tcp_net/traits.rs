use std::{io, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

pub trait TcpRead<S>
where
    S: Serialize + DeserializeOwned,
{
    /// Read a message and deserializs it into `M`.
    fn read(&mut self) -> Result<S, crate::tcp_net::error::PostcardFrameError> {
        self.read_custom()
    }

    fn read_custom<T>(&mut self) -> Result<T, crate::tcp_net::error::PostcardFrameError>
    where
        T: Serialize + DeserializeOwned,
    {
        Ok(postcard::from_bytes(&self.read_bytes()?)?)
    }

    /// Read a message.
    fn read_bytes(&mut self) -> Result<bytes::Bytes, crate::tcp_net::error::FrameError>;
}

pub trait TcpWrite<R>
where
    R: Serialize + DeserializeOwned,
{
    /// Serialize `M` and write it as a message.
    fn write(&mut self, msg: &R) -> Result<(), crate::tcp_net::error::PostcardFrameError> {
        self.write_custom(msg)
    }

    fn write_custom<T>(&mut self, msg: &T) -> Result<(), crate::tcp_net::error::PostcardFrameError>
    where
        T: Serialize + DeserializeOwned;

    /// Write a new message.
    fn write_bytes(&mut self, payload: &[u8]) -> Result<usize, crate::tcp_net::error::FrameError>;
}

pub trait TcpSocketOptions {
    fn flush(&mut self) -> io::Result<()>;
    fn shutdown(&mut self, how: std::net::Shutdown) -> io::Result<()>;

    fn nodelay(&self) -> io::Result<bool>;
    fn set_nodelay(&self, nodelay: bool) -> io::Result<()>;

    fn write_timeout(&self) -> io::Result<Option<Duration>>;
    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn read_timeout(&self) -> std::io::Result<Option<Duration>>;
    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()>;
}
