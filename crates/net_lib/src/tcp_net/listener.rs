use std::{
    io,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::tcp_net::TcpNet;

#[derive(Debug)]
pub struct TcpListener<R, S> {
    listener: std::net::TcpListener,

    request_phantom: PhantomData<R>,
    response_phantom: PhantomData<S>,
}

impl<R, S> TcpListener<R, S>
where
    R: Serialize + DeserializeOwned,
    S: Serialize + DeserializeOwned,
{
    pub fn bind<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        Ok(Self {
            listener: std::net::TcpListener::bind(addr)?,

            request_phantom: PhantomData,
            response_phantom: PhantomData,
        })
    }

    pub fn accept(&mut self) -> io::Result<(TcpNet<R, S>, SocketAddr)> {
        let (tcp_stream, addr) = self.listener.accept()?;
        Ok((TcpNet::from_stream(tcp_stream), addr))
    }

    pub fn incoming(&self) -> impl Iterator<Item = Result<TcpNet<R, S>, io::Error>> {
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
