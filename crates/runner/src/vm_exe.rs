use std::collections::BTreeMap;
use std::path::Path;

use crate::elf::Elf;
use crate::error::Result;
use crate::program::Program;
use crate::transpiler;

pub type SparseMemoryImage = BTreeMap<(u32, u32), u8>;

const MAX_GUEST_MEMORY: u32 = 1 << 29;

#[derive(Clone, Debug)]
pub struct VmExe {
    pub program: Program,
    pub pc_start: u32,
    pub init_memory: SparseMemoryImage,
}

impl VmExe {
    pub fn new(program: Program, pc_start: u32, init_memory: SparseMemoryImage) -> Self {
        let res = Self {
            program,
            pc_start,
            init_memory,
        };
        tracing::debug!("VmExe: {:#?}", res);
        res
    }

    /// Load a VmExe from an ELF file at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ELF file
    ///
    /// # Returns
    ///
    /// Returns a Result containing the VmExe or an error if the ELF file
    /// could not be loaded or transpiled.
    pub fn from_path(path: &Path) -> Result<Self> {
        let elf = Elf::from_path(path, MAX_GUEST_MEMORY)?;
        transpiler::transpile_elf(elf)
    }
}
