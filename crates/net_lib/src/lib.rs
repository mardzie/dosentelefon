//! # net_lib
//!
//! net_lib is a simple but powerful thin networking crate.

#[cfg(feature = "tcp")]
pub mod tcp_net;
#[cfg(feature = "udp")]
pub mod udp_net;
