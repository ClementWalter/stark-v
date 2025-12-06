use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use eyre::{bail, Context, Result};
use serde::Deserialize;

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
        bail!("missing Cargo.toml at {}", manifest_path.display());
    }
    let manifest_contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: CargoManifest = toml::from_str(&manifest_contents)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let status = Command::new("cargo")
        .current_dir(guest_dir)
        .env("RUSTUP_TOOLCHAIN", TOOLCHAIN)
        .args(["build", "--release", "--target", GUEST_TARGET])
        .status()
        .with_context(|| format!("building guest at {}", guest_dir.display()))?;
    if !status.success() {
        bail!("guest build failed for {}", guest_dir.display());
    }

    let binary_name = &manifest.package.name;
    let elf_path = guest_dir
        .join("target")
        .join(GUEST_TARGET)
        .join(PROFILE_DIR)
        .join(binary_name);
    if !elf_path.exists() {
        bail!("expected guest ELF at {}", elf_path.display());
    }

    Ok(BuildOutput { elf_path })
}
