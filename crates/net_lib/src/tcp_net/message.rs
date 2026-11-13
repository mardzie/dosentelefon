use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<T>
where
    T: Serialize + DeserializeOwned,
{
    len: u32,
    message: MessageType<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MessageType<T>
where
    T: Serialize + DeserializeOwned,
{
    Handshake { version: u8 },
    Data(T),
}
