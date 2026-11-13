use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<T>
where
    T: Serialize + DeserializeOwned,
{
    len: u32,
    message: T,
}
