use net_lib::tcp_net::{TcpNet, error::PostcardFrameError, traits::TcpRead};
use protocol::{request::Request, response::Response};

#[derive(Debug)]
pub struct Connection {
    stream: TcpNet<Request, Response>,
}

impl Connection {
    pub fn new(stream: TcpNet<Request, Response>) -> Self {
        Self { stream }
    }

    pub fn handle(mut self) {
        loop {
            let request = match self.stream.read() {
                Ok(request) => request,
                Err(_) => return,
            };

            todo!()
        }
    }
}
