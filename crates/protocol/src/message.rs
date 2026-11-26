use serde::{Deserialize, Serialize};

/// Messages that one client can send another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Message {
    ForwardVoice(Vec<u8>),
}
