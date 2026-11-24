use std::{process::exit, sync::LazyLock};

use server::Server;
use tracing::error;

use crate::config::Config;

mod config;
mod error;

const CONFIG_PATH: &str = "./config.toml";
pub const CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::load_or_default(CONFIG_PATH).expect("Should be able to load or use the default config")
});

fn main() {
    let Ok(mut server) = Server::new(&CONFIG.addr) else {
        error!("Failed to bind the server to {}", CONFIG.addr);
        exit(1);
    };

    if server.handle().is_err() {
        error!("Handling server failed");
    };
}
