use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    thread::JoinHandle,
};

mod connection;

use net_lib::tcp_net::TcpListener;
use protocol::{request::Request, response::Response};

use crate::connection::Connection;

#[derive(Debug)]
pub struct Server {
    threads: HashMap<SocketAddr, JoinHandle<()>>,
    listener: TcpListener<Request, Response>,
}

impl Server {
    pub fn new<A>(addr: A) -> Result<Self, ()>
    where
        A: ToSocketAddrs,
    {
        let Ok(listener) = TcpListener::bind(addr) else {
            return Err(());
        };

        Ok(Self {
            threads: HashMap::new(),
            listener,
        })
    }

    pub fn handle(&mut self) -> Result<(), ()> {
        loop {
            let Ok((stream, addr)) = self.listener.accept() else {
                continue;
            };

            let connection = Connection::new(stream);
            let handle = std::thread::spawn(|| connection.handle());
            self.threads.insert(addr, handle);
        }
    }
}
