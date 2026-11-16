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
        error::{FrameError, PayloadTooLarge, PostcardFrameError},
        traits::{TcpRead, TcpWrite},
    },
};

use bytes::{Buf, BufMut, BytesMut};
pub use listener::TcpListener;
use serde::{Serialize, de::DeserializeOwned};

pub mod error;
pub mod listener;
pub mod traits;

/// Length field size.
pub const HEADER_SIZE: usize = size_of::<u32>();
/// Default max payload size.
pub const MAX_PAYLOAD_SIZE: u16 = u16::MAX; // 64 KiB
pub const MAX_PAYLOAD_SIZE_USIZE: usize = MAX_PAYLOAD_SIZE as usize;

#[derive(Debug)]
pub struct TcpNet<T> {
    stream: TcpStream,
    buf: BytesMut,

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
            buf: Self::new_buf(),

            message_phantom: PhantomData,
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self::from_stream(self.stream.try_clone()?))
    }

    fn write_frame<E>(
        &mut self,
        fill: impl FnOnce(&mut BytesMut) -> Result<(), E>,
    ) -> Result<usize, E>
    where
        E: From<FrameError> + From<io::Error>,
    {
        self.buf.clear();
        self.buf.put_u32(0); // Placeholder, patched below
        fill(&mut self.buf)?;

        let payload_len = self.buf.len() - HEADER_SIZE;
        self.validate_payload_len(payload_len)?;
        self.buf[..HEADER_SIZE].copy_from_slice(&(payload_len as u32).to_be_bytes());

        self.stream.write_all(&self.buf)?;
        Ok(self.buf.len())
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
    fn write(&mut self, message: &M) -> Result<(), PostcardFrameError> {
        self.write_frame(|buf| {
            postcard::to_io(message, buf.writer())?;
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
    fn read(&mut self) -> Result<M, PostcardFrameError> {
        Ok(postcard::from_bytes::<M>(self.read_bytes()?)?)
    }

    fn read_bytes(&mut self) -> Result<&[u8], FrameError> {
        self.buf.clear();
        // Header
        self.buf.resize(HEADER_SIZE, 0);
        self.stream.read_exact(&mut self.buf)?;
        let len = usize::try_from(self.buf.get_u32()).expect("`u32` must fit into `usize`");
        self.validate_payload_len(len)?;
        // Payload
        self.buf.resize(len, 0);
        self.stream.read_exact(&mut self.buf)?;

        Ok(&self.buf)
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
