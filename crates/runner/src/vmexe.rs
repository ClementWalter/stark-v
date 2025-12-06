use std::collections::BTreeMap;

use crate::program::Program;

pub type SparseMemoryImage = BTreeMap<(u32, u32), u8>;

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
        println!("VmExe: {:#?}", res);
        res
    }
}
