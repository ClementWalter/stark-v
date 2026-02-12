//! Build script for RV32 guest linking.
//!
//! Why: the runner reads fixed linker symbols (`__input_start`, `__output_data`,
//! `__halt_flag`, etc.) to wire guest I/O memory. We must provide a custom
//! linker script on riscv32 builds so those symbols are present in the ELF.

use std::path::Path;

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch != "riscv32" {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR");
    let linker_path = Path::new(&manifest_dir).join("linker.ld");

    println!("cargo:rustc-link-arg=-T{}", linker_path.display());
    println!("cargo:rerun-if-changed=linker.ld");
}
