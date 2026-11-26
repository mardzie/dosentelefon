use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::message::Message;

/// Responses that the server can answer or just send to the client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Response {
    /// A [`Message`] from another client.
    Message { from: SocketAddr, msg: Message },
    /// Acknowledges
    Ack(Ack),
    /// A `Request::Get` response.
    Get(Get),
    /// Errors
    Error(Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ack {
    /// Connect successful
    ConnectAck,
    /// Disconnect successful
    DisconnectAck,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Get {
    /// All known users.
    Users(Vec<SocketAddr>),
    /// All user info.
    User { addr: SocketAddr },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Error {
    /// The requested user is unknown.
    UnknownUser(SocketAddr),
    /// The requesting user is not connected.
    Unconnected,
}
