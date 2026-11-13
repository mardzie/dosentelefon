use std::{io, time::Duration};

pub trait SocketOptions {
    /// Returns the read timeout.
    ///
    /// When the timeout is `None`, then `read` calls will block indefinitely.
    fn read_timeout(&self) -> io::Result<Option<std::time::Duration>>;

    /// Sets the read timeout for this socket.
    ///
    /// When `None` is specified, `read` calls will block indefinitely.
    /// An error is returned if a zero [`Duration`] is passed to this method.
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    /// Returns the write timeout.
    ///
    /// When the timeout is `None`, then `send` calls will block indefinitely.
    fn write_timeout(&self) -> io::Result<Option<Duration>>;

    /// Sets the write timeout for this socket.
    ///
    /// When `None` is specified, `write` calls will block indefinitely.
    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    /// Returns the ttl (Time To Live (Max hops over routers)).
    fn ttl(&self) -> io::Result<u32>;

    /// Sets the ttl (Time To Live (Max hops over routers)) for every packet sent from this socket.
    fn set_ttl(&self, ttl: u32) -> io::Result<()>;

    /// Moves the socket in or out of nonblocking mode.
    ///
    /// This will result in `recv`, `recv_from`, `send`, `send_bytes`, `send_to` and `send_bytes_to` to become nonblocking.
    ///
    /// A call to one of the methods in nonblocking mode will result in [`std::io::ErrorKind::WouldBlock`] when the operation would normally block.
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
}
