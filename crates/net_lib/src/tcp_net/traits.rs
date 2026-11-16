use std::{io, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

pub trait TcpWrite<M>
where
    M: Serialize + DeserializeOwned,
{
    /// Serialize `M` and write it as a message.
    fn write(&mut self, message: &M) -> Result<(), crate::tcp_net::error::PostcardFrameError>;
    /// Write a new message.
    fn write_bytes(&mut self, payload: &[u8]) -> Result<usize, crate::tcp_net::error::FrameError>;
}

pub trait TcpRead<M>
where
    M: Serialize + DeserializeOwned,
{
    /// Read a message and deserializs it into `M`.
    fn read(&mut self) -> Result<M, crate::tcp_net::error::PostcardFrameError>;

    /// Read a message.
    fn read_bytes(&mut self) -> Result<&[u8], crate::tcp_net::error::FrameError>;
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
