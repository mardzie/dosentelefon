use std::{
    fmt::Debug,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use crossbeam::channel::{Receiver, Sender};
use net_lib::tcp_net::{
    TcpNet,
    traits::{TcpRead, TcpSocketOptions},
};
use protocol::{request::Request, response::Response};
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub struct Connection {
    stream: TcpNet<Request, Response>,
    commands: Receiver<Commands>,
    command_sender: Sender<Commands>,
    stop_signal: Arc<AtomicBool>,
}

impl Connection {
    pub fn new(stream: TcpNet<Request, Response>, channel_cap: usize) -> ConnectionHandle<()> {
        let (command_sender, commands) = crossbeam::channel::bounded(channel_cap);
        let stop_sender = command_sender.clone();
        let stop = move || {
            let _ = stop_sender.send(Commands::Stop);
        };
        let connection = Self {
            stream,
            commands,
            command_sender: command_sender.clone(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        };

        let handle = std::thread::spawn(move || connection.handle());
        ConnectionHandle::new(command_sender, stop, handle)
    }

    fn handle(mut self) {
        let peer_addr = self
            .stream
            .peer_addr()
            .expect("Should be able to get peer address");

        let handle = std::thread::spawn({
            let stream = self
                .stream
                .try_clone()
                .expect("Should be able to clone stream");
            let command_sender = self.command_sender.clone();
            let stop_signal = self.stop_signal.clone();

            move || Self::stream_reader(stream, command_sender, stop_signal, peer_addr)
        });

        'main_loop: loop {
            if self.stop_signal.load(Ordering::Acquire) {
                info!("Stopping {} connection...", peer_addr);
                break 'main_loop;
            }

            let command = match self.commands.recv() {
                Ok(command) => command,
                Err(_) => {
                    warn!("Command channel is closed");
                    self.stop_signal.store(true, Ordering::Release);
                    continue 'main_loop;
                }
            };

            match command {
                Commands::Received(request) => request,
                Commands::Stop => {
                    self.stop_signal.store(true, Ordering::Release);
                    continue 'main_loop;
                }
            };
        }

        if let Err(e) = self.stream.flush() {
            warn!("Failed to flush TcpStream while shutting down {}", e);
        }
        if let Err(e) = self.stream.shutdown(std::net::Shutdown::Both) {
            warn!("Failed to shutdown TcpStream: {}", e);
        }
        let _ = handle.join();
    }

    fn stream_reader(
        mut stream: TcpNet<Request, Response>,
        command_sender: Sender<Commands>,
        stop_signal: Arc<AtomicBool>,
        peer_addr: SocketAddr,
    ) {
        loop {
            if stop_signal.load(Ordering::Acquire) {
                return;
            }

            match stream.read() {
                Ok(req) => {
                    debug!("Connection {} got request: {:?}", peer_addr, req);
                    let _ = command_sender.send(Commands::Received(req));
                }
                Err(e) => {
                    error!("Connection got errounous frame {}", e);
                    let _ = command_sender.send(Commands::Stop);
                }
            };
        }
    }
}

pub struct ConnectionHandle<T> {
    command_sender: Sender<Commands>,
    stop: Box<dyn FnOnce()>,
    handle: JoinHandle<T>,
}

impl<T> ConnectionHandle<T> {
    #[must_use]
    pub fn new<F>(command_sender: Sender<Commands>, stop: F, handle: JoinHandle<T>) -> Self
    where
        F: FnOnce() + 'static,
    {
        Self {
            command_sender,
            stop: Box::new(stop),
            handle,
        }
    }

    pub fn send(&self, response: Response) -> Result<(), crossbeam::channel::SendError<Commands>> {
        self.command_sender.send(Commands::Send(response))
    }

    pub fn stop(self) -> std::thread::Result<T> {
        (self.stop)();
        self.handle.join()
    }
}

impl<T> Debug for ConnectionHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionData")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Commands {
    Send(Response),
    Received(Request),
    Stop,
}
