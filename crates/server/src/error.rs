#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
