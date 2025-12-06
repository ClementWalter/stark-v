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

/// Build the guest package with the risc0 RISC-V target and return the ELF path.
pub fn build_guest(guest_dir: &Path) -> Result<BuildOutput> {
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

    let binary_name = &manifest.package.name;
    let elf_path = guest_dir
        .join("target")
        .join(GUEST_TARGET)
        .join(PROFILE_DIR)
        .join(binary_name);
    if !elf_path.exists() {
        return Err(BuilderError::ExpectedGuestElf(elf_path));
    }

    Ok(BuildOutput { elf_path })
}
