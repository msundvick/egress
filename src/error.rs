use failure::Fail;

/// Possible failure modes for creating and closing `Egress` contexts.
#[derive(Debug, Fail)]
pub enum ErrorKind {
    /// Wrapper for `std::io::Error` from file operations.
    #[fail(display = "I/O error: {}", _0)]
    IoError(#[cause] std::io::Error),

    /// Wrapper for `toml` serialization errors for config data.
    #[fail(display = "error while serializing config data to TOML: {}", _0)]
    TomlSerError(#[cause] toml::ser::Error),

    /// Wrapper for `toml` deserialization errors for config data.
    #[fail(display = "error while deserializing config data from TOML: {}", _0)]
    TomlDeError(#[cause] toml::de::Error),

    /// Wrapper for errors caused by serializing/deserializing artifacts from JSON.
    #[fail(display = "error while (de)serializing artifact from JSON: {}", _0)]
    JsonError(#[cause] serde_json::error::Error),
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        ErrorKind::IoError(err)
    }
}

impl From<toml::ser::Error> for ErrorKind {
    fn from(err: toml::ser::Error) -> Self {
        ErrorKind::TomlSerError(err)
    }
}

impl From<toml::de::Error> for ErrorKind {
    fn from(err: toml::de::Error) -> Self {
        ErrorKind::TomlDeError(err)
    }
}

impl From<serde_json::error::Error> for ErrorKind {
    fn from(err: serde_json::error::Error) -> Self {
        ErrorKind::JsonError(err)
    }
}
