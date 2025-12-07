use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

mod error;

pub use error::{BuilderError, Result};

const GUEST_TARGET: &str = "riscv32im-risc0-zkvm-elf";
const TOOLCHAIN: &str = "risc0";
const PROFILE_DIR: &str = "release";

#[derive(Debug)]
pub struct BuildOutput {
    pub elf_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoManifest {
    package: ManifestPackage,
}

#[derive(Debug, Deserialize)]
struct ManifestPackage {
    name: String,
}

/// Parse the Cargo.toml manifest and extract the package name.
fn parse_manifest(guest_dir: &Path) -> Result<String> {
    let manifest_path = guest_dir.join("Cargo.toml");
    if !manifest_path.exists() {
        return Err(BuilderError::MissingCargoToml(manifest_path));
    }
    let manifest_contents =
        fs::read_to_string(&manifest_path).map_err(|source| BuilderError::ReadFile {
            path: manifest_path.clone(),
            source,
        })?;
    let manifest: CargoManifest =
        toml::from_str(&manifest_contents).map_err(|source| BuilderError::ParseToml {
            path: manifest_path.clone(),
            source,
        })?;
    Ok(manifest.package.name)
}

/// Compute the expected ELF path for a given package name.
fn compute_elf_path(guest_dir: &Path, binary_name: &str) -> PathBuf {
    guest_dir
        .join("target")
        .join(GUEST_TARGET)
        .join(PROFILE_DIR)
        .join(binary_name)
}

/// Verify the ELF file exists at the expected path.
fn verify_elf_exists(elf_path: &Path) -> Result<()> {
    if !elf_path.exists() {
        return Err(BuilderError::ExpectedGuestElf(elf_path.to_path_buf()));
    }
    Ok(())
}

/// Build the guest package with the risc0 RISC-V target and return the ELF path.
pub fn build_guest(guest_dir: &Path) -> Result<BuildOutput> {
    let binary_name = parse_manifest(guest_dir)?;

    let status = Command::new("cargo")
        .current_dir(guest_dir)
        .env("RUSTUP_TOOLCHAIN", TOOLCHAIN)
        .args(["build", "--release", "--target", GUEST_TARGET])
        .status()
        .map_err(|source| BuilderError::BuildGuest {
            path: guest_dir.to_path_buf(),
            source,
        })?;
    if !status.success() {
        return Err(BuilderError::GuestBuildFailed(guest_dir.to_path_buf()));
    }

    let elf_path = compute_elf_path(guest_dir, &binary_name);
    verify_elf_exists(&elf_path)?;

    Ok(BuildOutput { elf_path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_build_output_debug() {
        let output = BuildOutput {
            elf_path: PathBuf::from("/path/to/elf"),
        };
        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("BuildOutput"));
        assert!(debug_str.contains("elf_path"));
    }

    #[test]
    fn test_build_guest_missing_cargo_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = build_guest(temp_dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::MissingCargoToml(_)));
    }

    #[test]
    fn test_build_guest_invalid_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Write invalid TOML
        let mut file = std::fs::File::create(&cargo_toml_path).unwrap();
        file.write_all(b"this is not valid toml [[[").unwrap();

        let result = build_guest(temp_dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::ParseToml { .. }));
    }

    #[test]
    fn test_build_guest_missing_package_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Write TOML without package.name
        let mut file = std::fs::File::create(&cargo_toml_path).unwrap();
        file.write_all(b"[dependencies]\n").unwrap();

        let result = build_guest(temp_dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::ParseToml { .. }));
    }

    #[test]
    fn test_cargo_manifest_deserialize() {
        let toml_str = r#"
            [package]
            name = "test-package"
            version = "0.1.0"
        "#;

        let manifest: CargoManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test-package");
    }

    #[test]
    fn test_cargo_manifest_deserialize_with_extra_fields() {
        let toml_str = r#"
            [package]
            name = "test-package"
            version = "0.1.0"
            edition = "2021"
            authors = ["Test Author"]

            [dependencies]
            serde = "1.0"
        "#;

        let manifest: CargoManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test-package");
    }

    #[test]
    fn test_constants() {
        assert_eq!(GUEST_TARGET, "riscv32im-risc0-zkvm-elf");
        assert_eq!(TOOLCHAIN, "risc0");
        assert_eq!(PROFILE_DIR, "release");
    }

    #[test]
    fn test_build_guest_cargo_not_found() {
        // Test the BuildGuest error path by using an invalid cargo command
        // This is hard to test directly, but we can test via a non-existent directory
        // that would fail at the command execution stage
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Write valid TOML so we get past the parse stage
        let mut file = std::fs::File::create(&cargo_toml_path).unwrap();
        file.write_all(b"[package]\nname = \"test-guest\"\nversion = \"0.1.0\"\n")
            .unwrap();

        // The build will fail because risc0 toolchain is not installed
        // This tests the GuestBuildFailed path
        let result = build_guest(temp_dir.path());
        assert!(result.is_err());
        // Either BuildGuest (cargo not found) or GuestBuildFailed (cargo failed)
        let err = result.unwrap_err();
        assert!(
            matches!(err, BuilderError::BuildGuest { .. })
                || matches!(err, BuilderError::GuestBuildFailed(_))
        );
    }

    #[test]
    fn test_build_guest_elf_not_found() {
        // This tests the ExpectedGuestElf error path
        // We need to simulate a successful build but missing ELF
        // Since we can't actually run cargo, we test the error type directly
        let err = BuilderError::ExpectedGuestElf(PathBuf::from("/path/to/missing.elf"));
        assert!(format!("{}", err).contains("expected guest ELF"));
    }

    #[test]
    fn test_manifest_package_debug() {
        let toml_str = r#"
            [package]
            name = "test-package"
            version = "0.1.0"
        "#;
        let manifest: CargoManifest = toml::from_str(toml_str).unwrap();
        let debug_str = format!("{:?}", manifest);
        assert!(debug_str.contains("CargoManifest"));
        assert!(debug_str.contains("test-package"));

        let pkg_debug = format!("{:?}", manifest.package);
        assert!(pkg_debug.contains("ManifestPackage"));
    }

    #[test]
    fn test_parse_manifest_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        let mut file = std::fs::File::create(&cargo_toml_path).unwrap();
        file.write_all(b"[package]\nname = \"my-guest\"\nversion = \"0.1.0\"\n")
            .unwrap();

        let result = parse_manifest(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "my-guest");
    }

    #[test]
    fn test_compute_elf_path() {
        let guest_dir = Path::new("/home/user/project");
        let elf_path = compute_elf_path(guest_dir, "my-binary");
        assert_eq!(
            elf_path,
            PathBuf::from("/home/user/project/target/riscv32im-risc0-zkvm-elf/release/my-binary")
        );
    }

    #[test]
    fn test_verify_elf_exists_missing() {
        let result = verify_elf_exists(Path::new("/nonexistent/path/to/elf"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::ExpectedGuestElf(_)));
    }

    #[test]
    fn test_verify_elf_exists_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");

        // Create the file
        let mut file = std::fs::File::create(&elf_path).unwrap();
        file.write_all(b"fake elf content").unwrap();

        let result = verify_elf_exists(&elf_path);
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_manifest_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        // Create file with no read permissions
        let mut file = std::fs::File::create(&cargo_toml_path).unwrap();
        file.write_all(b"[package]\nname = \"test\"\n").unwrap();
        std::fs::set_permissions(&cargo_toml_path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = parse_manifest(temp_dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::ReadFile { .. }));

        // Restore permissions for cleanup
        std::fs::set_permissions(&cargo_toml_path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    // Note: Full integration test for build_guest requires risc0 toolchain
    // which may not be available in CI. These tests cover error paths.
}
