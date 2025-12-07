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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_cargo_toml_display() {
        let err = BuilderError::MissingCargoToml(PathBuf::from("/path/to/Cargo.toml"));
        assert_eq!(format!("{}", err), "missing Cargo.toml at /path/to/Cargo.toml");
    }

    #[test]
    fn test_read_file_display() {
        let err = BuilderError::ReadFile {
            path: PathBuf::from("/path/to/file"),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("reading /path/to/file"));
    }

    #[test]
    fn test_parse_toml_display() {
        // Create a parse error by parsing invalid TOML
        let parse_result: std::result::Result<toml::Value, _> = toml::from_str("[[[invalid");
        let source = parse_result.unwrap_err();
        let err = BuilderError::ParseToml {
            path: PathBuf::from("/path/to/Cargo.toml"),
            source,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("parsing /path/to/Cargo.toml"));
    }

    #[test]
    fn test_build_guest_display() {
        let err = BuilderError::BuildGuest {
            path: PathBuf::from("/path/to/guest"),
            source: io::Error::new(io::ErrorKind::NotFound, "cargo not found"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("building guest at /path/to/guest"));
    }

    #[test]
    fn test_guest_build_failed_display() {
        let err = BuilderError::GuestBuildFailed(PathBuf::from("/path/to/guest"));
        assert_eq!(format!("{}", err), "guest build failed for /path/to/guest");
    }

    #[test]
    fn test_expected_guest_elf_display() {
        let err = BuilderError::ExpectedGuestElf(PathBuf::from("/path/to/elf"));
        assert_eq!(format!("{}", err), "expected guest ELF at /path/to/elf");
    }

    #[test]
    fn test_error_debug_impl() {
        let err = BuilderError::GuestBuildFailed(PathBuf::from("/path/to/guest"));
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("GuestBuildFailed"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(BuilderError::GuestBuildFailed(PathBuf::from("/path")));
        assert!(result.is_err());
    }
}
