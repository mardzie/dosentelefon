use std::{
    io,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::tcp_net::TcpNet;

#[derive(Debug)]
pub struct TcpListener<M> {
    listener: std::net::TcpListener,

    message_phantom: PhantomData<M>,
}

impl<M> TcpListener<M>
where
    M: Serialize + DeserializeOwned,
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

    pub fn accept(&mut self) -> io::Result<(TcpNet<M>, SocketAddr)> {
        let (tcp_stream, addr) = self.listener.accept()?;
        Ok((TcpNet::from_stream(tcp_stream), addr))
    }

    pub fn incoming(&self) -> impl Iterator<Item = Result<TcpNet<M>, io::Error>> {
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
