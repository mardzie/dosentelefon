use std::{
    io,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::tcp_net::TcpNet;

#[derive(Debug)]
pub struct TcpListener<T> {
    listener: std::net::TcpListener,

    message_phantom: PhantomData<T>,
}

impl<T> TcpListener<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn bind<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        Ok(Self {
            listener: std::net::TcpListener::bind(addr)?,

            message_phantom: PhantomData,
        })
    }

    pub fn accept(&mut self) -> io::Result<(TcpNet<T>, SocketAddr)> {
        let (tcp_stream, addr) = self.listener.accept()?;
        Ok((TcpNet::from_stream(tcp_stream), addr))
    }

    pub fn incoming(&self) -> impl Iterator<Item = Result<TcpNet<T>, io::Error>> {
        self.listener
            .incoming()
            .map(|res| res.map(|stream| TcpNet::from_stream(stream)))
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.listener.set_nonblocking(nonblocking)
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
