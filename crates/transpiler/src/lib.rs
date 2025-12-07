mod elf;
mod error;
mod instruction;
mod program;
mod transpiler;
mod vm_exe;

pub use error::{Result, RunnerError};
pub use instruction::Instruction;
pub use program::Program;
pub use vm_exe::{SparseMemoryImage, VmExe};

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
        let exe = VmExe::from_path(&elf_path)?;
        assert!(!exe.program.is_empty());
        assert!(!exe.init_memory.is_empty());
        Ok(())
    }
}
