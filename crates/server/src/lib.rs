use std::{
    collections::HashMap,
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self},
    time::Duration,
};

use parking_lot::RwLock;

mod connection;

use net_lib::tcp_net::TcpListener;
use protocol::{request::Request, response::Response};
use tracing::{debug, info, warn};

use crate::connection::{Connection, ConnectionHandle};

pub const SLEEP_DUR: Duration = Duration::from_millis(10);
const CHANNEL_CAPACITY: usize = 8;

#[derive(Debug)]
pub struct Server {
    handles: Arc<RwLock<HashMap<SocketAddr, ConnectionHandle<()>>>>,
    listener: TcpListener<Request, Response>,
    stop_signal: AtomicBool,
}

impl Server {
    pub fn new<A>(addr: A) -> std::io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            listener,
            stop_signal: AtomicBool::new(false),
        })
    }

    pub fn handle(&mut self) -> std::io::Result<()> {
        let e: Option<std::io::Error> = loop {
            if self.stop_signal.load(Ordering::Acquire) {
                break None;
            }

            let (stream, addr) = match self.listener.accept() {
                Ok(incoming) => incoming,
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::Interrupted => {
                        thread::park_timeout(SLEEP_DUR);
                        continue;
                    }
                    _ => break Some(e),
                },
            };

            // TODO: Give the Connections access to all handles.
            todo!();
            let handle = Connection::new(stream, CHANNEL_CAPACITY);
            self.handles.write().insert(addr, handle);
        };

        info!("Stopping Server");
        for (addr, conn_data) in self.handles.write().drain() {
            debug!("Stopping connection from {}", addr);
            match conn_data.stop() {
                Ok(_) => {
                    info!("Connection shutdown: {}", addr)
                }
                Err(e) => {
                    warn!("Failed to stop connection {} properly: {:?}", addr, e)
                }
            }
        }

        e.map_or(Ok(()), |e| Err(e))
    }
}
