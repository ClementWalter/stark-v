mod elf;
mod instruction;
mod program;
mod transpiler;
mod vmexe;

pub use instruction::{DebugInfo, Instruction};
pub use program::Program;
pub use vmexe::{SparseMemoryImage, VmExe};

use std::path::Path;

use eyre::Result;

/// Convenience function to load a VmExe from an ELF file.
///
/// This function is a wrapper around `VmExe::from_path` for backward compatibility.
pub fn load_vmexe_from_elf(path: &Path) -> Result<VmExe> {
    VmExe::from_path(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    #[ignore]
    fn load_playground_guest() -> Result<()> {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let elf_path = workspace_root
            .join("guests/playground/target/riscv32im-risc0-zkvm-elf/release/playground");
        if !elf_path.exists() {
            return Ok(());
        }
        let exe = load_vmexe_from_elf(&elf_path)?;
        assert!(!exe.program.is_empty());
        assert!(!exe.init_memory.is_empty());
        Ok(())
    }
}
