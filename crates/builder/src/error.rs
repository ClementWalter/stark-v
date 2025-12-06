use std::io;
use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BuilderError>;

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("missing Cargo.toml at {0}")]
    MissingCargoToml(PathBuf),

    #[error("reading {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("parsing {path}: {source}")]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("building guest at {path}: {source}")]
    BuildGuest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("guest build failed for {0}")]
    GuestBuildFailed(PathBuf),

    #[error("expected guest ELF at {0}")]
    ExpectedGuestElf(PathBuf),
}
