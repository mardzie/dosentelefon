use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::message::Message;

/// Requests that a client can send to the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Request {
    /// A [`Message`] forward request from a client.
    Message {
        to: SocketAddr,
        msg: Message,
    },
    Syn(Syn),
    /// A `Get` request.
    Get(Get),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Syn {
    /// A connect request.
    Connect,
    /// A disconnect request.
    Disconnect,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Get {
    /// Get all users.
    Users,
    /// Get user info for one user.
    User(SocketAddr),
}
