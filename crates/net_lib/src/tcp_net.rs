//! TCP related types and traits.
//!
//! This module provides TCP networking types.

use std::{
    io::{self, Read, Write},
    marker::PhantomData,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{
    macros::{socket_options, tcp_socket_options},
    tcp_net::{
        error::{FrameError, IsRetryable, PayloadTooLarge},
        traits::{TcpRead, TcpWrite},
    },
};

use bytes::{Buf, BufMut, BytesMut};
pub use listener::TcpListener;
use serde::{Serialize, de::DeserializeOwned};

pub mod error;
pub mod listener;
pub mod traits;

type Header = u16;
/// Length field size.
pub const HEADER_SIZE: usize = size_of::<Header>();
/// Default max payload size.
pub const MAX_PAYLOAD_SIZE: Header = Header::MAX; // 64 KiB
pub const MAX_PAYLOAD_SIZE_USIZE: usize = MAX_PAYLOAD_SIZE as usize;

#[derive(Debug)]
pub struct TcpNet<T> {
    stream: TcpStream,
    rbuf: BytesMut,
    poisoned: bool,
    message_phantom: PhantomData<T>,
}

impl<T> TcpNet<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn connect<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr)?;
        Ok(Self::from_stream(stream))
    }

    pub fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        let stream = TcpStream::connect_timeout(addr, timeout)?;
        Ok(Self::from_stream(stream))
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            stream,
            rbuf: Self::new_buf(),
            poisoned: false,

            message_phantom: PhantomData,
        }
    }

    /// Try to clone the underlying stream.
    ///
    /// `R` allows one side to have another message type than the other.
    /// This allows for one side to be a writer and write a request type
    /// while the other is a reader and reads a response type.
    pub fn try_clone<R>(&self) -> io::Result<TcpNet<R>>
    where
        R: Serialize + DeserializeOwned,
    {
        Ok(TcpNet::<R>::from_stream(self.stream.try_clone()?))
    }

    fn write_frame<E>(
        &mut self,
        fill: impl FnOnce(&mut BytesMut) -> Result<(), E>,
    ) -> Result<usize, E>
    where
        E: From<FrameError> + From<io::Error>,
    {
        self.rbuf.clear();
        self.rbuf.put_u16(0); // Placeholder, patched below
        fill(&mut self.rbuf)?;

        let payload_len = self.rbuf.len() - HEADER_SIZE;
        self.validate_payload_len(payload_len)?;
        self.rbuf[..HEADER_SIZE].copy_from_slice(&(payload_len as u16).to_be_bytes());

        self.stream.write_all(&self.rbuf)?;
        Ok(self.rbuf.len())
    }

    fn read_frame(&mut self) -> Result<BytesMut, FrameError> {
        if self.poisoned {
            return Err(FrameError::Poisoned);
        }

        let res = self.read_frame_inner();
        if matches!(&res, Err(e) if !e.is_retryable()) {
            self.poisoned = true;
        }
        res
    }

    fn read_frame_inner(&mut self) -> Result<BytesMut, FrameError> {
        loop {
            if let Some(frame) = self.try_decode()? {
                return Ok(frame);
            }
            if self.fill()? == 0 {
                return Err(if self.rbuf.is_empty() {
                    FrameError::Closed
                } else {
                    FrameError::UnexpectedEOF
                });
            }
        }
    }

    /// Tries to decode `rbuf`.
    ///
    /// Operates only on `rbuf`.
    ///
    /// When `None` is returned it needs more bytes.
    fn try_decode(&mut self) -> Result<Option<BytesMut>, FrameError> {
        if self.rbuf.len() < HEADER_SIZE {
            return Ok(None);
        }
        let len = usize::from(u16::from_be_bytes([self.rbuf[0], self.rbuf[1]]));
        self.validate_payload_len(len)?;
        let total = HEADER_SIZE + len;
        if self.rbuf.len() < total {
            self.rbuf.reserve(total - self.rbuf.len());
            return Ok(None);
        }
        self.rbuf.advance(HEADER_SIZE);
        Ok(Some(self.rbuf.split_to(len))) // O(1); shares the allocation
    }

    fn fill(&mut self) -> io::Result<usize> {
        let mut tmp = [0u8; 4 * 1024];
        loop {
            match self.stream.read(&mut tmp) {
                Ok(n) => {
                    self.rbuf.extend_from_slice(&tmp[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }

    #[must_use]
    fn new_buf() -> BytesMut {
        BytesMut::with_capacity(MAX_PAYLOAD_SIZE_USIZE)
    }

    fn validate_payload_len(&self, len: usize) -> Result<(), FrameError> {
        if !self.payload_len_in_range(len) {
            return Err(FrameError::PayloadTooLarge(PayloadTooLarge {
                len: len,
                max: MAX_PAYLOAD_SIZE_USIZE,
            }));
        }

        Ok(())
    }

    #[must_use]
    fn payload_len_in_range(&self, len: usize) -> bool {
        len + HEADER_SIZE <= MAX_PAYLOAD_SIZE_USIZE
    }
}

impl<M> TcpWrite<M> for TcpNet<M>
where
    M: Serialize + DeserializeOwned,
{
    fn write_custom<T>(&mut self, msg: &T) -> Result<(), crate::tcp_net::error::PostcardFrameError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.write_frame(|buf| {
            postcard::to_io(msg, buf.writer())?;
            Ok(())
        })
        .map(drop)
    }

    fn write_bytes(&mut self, payload: &[u8]) -> Result<usize, FrameError> {
        self.write_frame(|buf| {
            buf.put_slice(&payload);
            Ok(())
        })
    }
}

impl<M> TcpRead<M> for TcpNet<M>
where
    M: Serialize + DeserializeOwned,
{
    fn read_bytes(&mut self) -> Result<bytes::Bytes, FrameError> {
        Ok(self.read_frame()?.freeze())
    }
}

socket_options!(TcpNet, stream, #[inline]);

tcp_socket_options!(TcpNet, stream, #[inline]);

#[cfg(test)]
mod tcp_test {
    use serde::{Deserialize, Serialize};

    use crate::tcp_net::{
        TcpListener, TcpNet,
        traits::{TcpRead, TcpWrite},
    };

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
    struct Message(pub String);

    impl From<&str> for Message {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }

    impl From<String> for Message {
        fn from(value: String) -> Self {
            Self(value)
        }
    }

    fn get_streams() -> (TcpNet<Message>, TcpNet<Message>) {
        const LISTENER_ADDR: &str = "localhost:0";

        let mut listener = TcpListener::<Message>::bind(LISTENER_ADDR).unwrap();
        let stream1 = TcpNet::<Message>::connect(listener.local_addr().unwrap()).unwrap();
        let (stream2, _) = listener.accept().unwrap();
        (stream1, stream2)
    }

    #[test]
    fn pass_message() {
        let request = Message::from("Request");
        let response = Message::from("Response");

        let (mut send_stream, mut recv_stream) = get_streams();
        send_stream.write(&request).unwrap();
        assert_eq!(recv_stream.read().unwrap(), request);
        recv_stream.write(&response).unwrap();
        assert_eq!(send_stream.read().unwrap(), response);
    }

    #[test]
    fn pass_raw_message() {
        let request = postcard::to_stdvec(&Message::from("Request")).unwrap();
        let response = postcard::to_stdvec(&Message::from("Response")).unwrap();

        let (mut send_stream, mut recv_stream) = get_streams();
        send_stream.write_bytes(&request).unwrap();
        assert_eq!(recv_stream.read_bytes().unwrap(), request);
        recv_stream.write_bytes(&response).unwrap();
        assert_eq!(send_stream.read_bytes().unwrap(), response);
    }
}
