use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    pub addr: String,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config_toml = fs::read_to_string(path)?;
        toml::from_str(&config_toml).map_err(ConfigError::from)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        match Self::load(&path) {
            Ok(config) => Ok(config),
            Err(ConfigError::Io(_)) => {
                let config = Self::default();
                config.write(&path)?;
                Ok(config)
            }
            Err(e) => Err(e),
        }
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        if !fs::exists(&path)? {
            fs::create_dir_all(&path)?;
        }

        let config = toml::to_string_pretty(self)?;
        fs::write(&path, config)?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: "localhost:0".to_string(),
        }
    }
}
