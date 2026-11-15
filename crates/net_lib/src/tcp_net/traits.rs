use std::{io, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

pub trait TcpWrite<M>
where
    M: Serialize + DeserializeOwned,
{
    fn write(&mut self, message: M) -> Result<usize, crate::tcp_net::error::PostcardFrameError>;
    fn write_bytes(&mut self, buf: &[u8]) -> Result<usize, crate::tcp_net::error::FrameError>;
    fn write_all(&mut self, buf: &[u8]) -> Result<(), crate::tcp_net::error::FrameError>;
}

pub trait TcpRead<M>
where
    M: Serialize + DeserializeOwned,
{
    fn read(&mut self) -> Result<M, crate::tcp_net::error::PostcardFrameError>;
    fn read_bytes(&mut self) -> Result<&[u8], crate::tcp_net::error::FrameError>;
    fn read_exact(&mut self) -> Result<&[u8], crate::tcp_net::error::FrameError>;
}

pub trait TcpSocketOptions {
    fn nodelay(&self) -> io::Result<bool>;
    fn set_nodelay(&self, nodelay: bool) -> io::Result<()>;

    fn flush(&mut self) -> io::Result<()>;

    fn shutdown(&mut self, how: std::net::Shutdown) -> io::Result<()>;

    fn write_timeout(&self) -> io::Result<Option<Duration>>;
    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn read_timeout(&self) -> std::io::Result<Option<Duration>>;
    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()>;
}
