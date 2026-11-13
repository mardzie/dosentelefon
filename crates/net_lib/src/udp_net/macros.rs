/// Generates an implementation of `BufOps`.
///
/// # Syntax:
///
/// This will implement `BufOps` with truncation.
/// This means the buffer has a truncation detection byte.
/// ```text
/// buf_ops!(name: ident, buf: ident);
/// ```
/// or
/// ```text
/// buf_ops!(name: ident, buf: ident, true);
/// ```
///
/// When appending `false` the macro will generate a impl for `BufOps` without truncation.
/// This means the buffer has no truncation detection byte.
/// ```text
/// buf_ops!(name: ident, buf: ident, false);
/// ```
macro_rules! buf_ops {
    (
        // The name of the struct on which `BufOps` will get implemented.
        $name:ident,
        $buf:ident) => {
        buf_ops!($name, $buf, true);
    };

    ($name:ident, $buf:ident, true) => {
        impl<P> $crate::udp_net::BufOps for $name<P>
        where
            P: ::serde::Serialize + ::serde::de::DeserializeOwned,
        {
            /// Get the current usable buffer length.
            #[inline]
            fn buf_len(&self) -> usize {
                self.$buf.len() - $crate::udp_net::TRUNCATION_BYTE
            }

            /// Resize the buffer to the `new_len` of usable bytes.
            /// This will either expand or shrink the buffer.
            ///
            /// This operation can be expensive.
            /// Use with caution.
            fn resize_buf(&mut self, new_len: usize) {
                assert!(new_len > 0);
                $crate::udp_net::resize_buffer(
                    &mut self.$buf,
                    new_len + $crate::udp_net::TRUNCATION_BYTE,
                );
            }
        }
    };

    ($name:ident, $buf:ident, false) => {
        impl<P> $crate::udp_net::BufOps for $name<P>
        where
            P: ::serde::Serialize + ::serde::de::DeserializeOwned,
        {
            /// Returns the current buffer length.
            #[inline]
            fn buf_len(&self) -> usize {
                self.$buf.len()
            }

            /// Resize the buffer to the `new_len`.
            /// This will either expand or shrink the buffer.
            ///
            /// This operation can be expensive.
            /// Use with caution.
            fn resize_buf(&mut self, new_len: usize) {
                assert!(new_len > 0);
                $crate::udp_net::resize_buffer(&mut self.$buf, new_len);
            }
        }
    };
}
pub(crate) use buf_ops;
