/// Generates an impl for `SocketOptions`.
///
/// # Syntax:
///
/// ```rust ignore
/// socket_options!(name: ident, inner: ident);
/// socket_options!(name: ident, inner: ident, #[inline]);
/// socket_options!(name: ident, inner: ident, #[inline], #[must_use]); // `#[must_use]` doesn't make much sense here.
/// ```
macro_rules! socket_options {
    ($name:ident, $inner:ident $(, #[$attr:meta])*) => {
        impl<P> $crate::traits::SocketOptions for $name<P>
        where
            P: ::serde::Serialize + ::serde::de::DeserializeOwned,
        {
            $(#[$attr])*
            fn read_timeout(&self) -> ::std::io::Result<Option<std::time::Duration>> {
                self.$inner.read_timeout()
            }

            $(#[$attr])*
            fn set_read_timeout(
                &self,
                dur: Option<::std::time::Duration>,
            ) -> ::std::io::Result<()> {
                self.$inner.set_read_timeout(dur)
            }

            $(#[$attr])*
            fn write_timeout(&self) -> ::std::io::Result<Option<::std::time::Duration>> {
                self.$inner.write_timeout()
            }

            $(#[$attr])*
            fn set_write_timeout(
                &self,
                dur: Option<::std::time::Duration>,
            ) -> ::std::io::Result<()> {
                self.$inner.set_write_timeout(dur)
            }

            $(#[$attr])*
            fn ttl(&self) -> ::std::io::Result<u32> {
                self.$inner.ttl()
            }

            $(#[$attr])*
            fn set_ttl(&self, ttl: u32) -> ::std::io::Result<()> {
                self.$inner.set_ttl(ttl)
            }

            $(#[$attr])*
            fn set_nonblocking(&self, nonblocking: bool) -> ::std::io::Result<()> {
                self.$inner.set_nonblocking(nonblocking)
            }
        }
    };
}
pub(crate) use socket_options;

macro_rules! tcp_socket_options {
    ($name:ident, $inner:ident $(, #[$attr:meta])*) => {
        impl<R, W> $crate::tcp_net::traits::TcpSocketOptions for $name<R, W>
        where
            R: ::serde::Serialize + ::serde::de::DeserializeOwned,
            W: ::serde::Serialize + ::serde::de::DeserializeOwned,
        {
            $(#[$attr])*
            fn nodelay(&self) -> io::Result<bool> {
                self.$inner.nodelay()
            }

            $(#[$attr])*
            fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
                self.$inner.set_nodelay(nodelay)
            }

            $(#[$attr])*
            fn flush(&mut self) -> io::Result<()> {
                self.$inner.flush()
            }

            $(#[$attr])*
            fn shutdown(&mut self, how: std::net::Shutdown) -> io::Result<()> {
                self.$inner.shutdown(how)
            }

            $(#[$attr])*
            fn write_timeout(&self) -> io::Result<Option<Duration>> {
                self.$inner.write_timeout()
            }

            $(#[$attr])*
            fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
                self.$inner.set_write_timeout(dur)
            }

            $(#[$attr])*
            fn read_timeout(&self) -> std::io::Result<Option<Duration>> {
                self.$inner.read_timeout()
            }

            $(#[$attr])*
            fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
                self.$inner.set_read_timeout(dur)
            }
        }
    };
}
pub(crate) use tcp_socket_options;
