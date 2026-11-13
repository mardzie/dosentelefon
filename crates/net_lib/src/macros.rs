/// Generates an impl for `SocketOptions`.
///
/// # Syntax:
///
/// ```rust norun
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
