use crate::instruction::{DebugInfo, Instruction};

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub instructions_and_debug_infos: Vec<Option<(Instruction, Option<DebugInfo>)>>,
    pub pc_base: u32,
}

impl Program {
    pub fn from_instructions(instructions: Vec<Instruction>, pc_base: u32) -> Self {
        Self {
            instructions_and_debug_infos: instructions
                .into_iter()
                .map(|instruction| Some((instruction, None)))
                .collect(),
            pc_base,
        }
    }

    pub fn len(&self) -> usize {
        self.instructions_and_debug_infos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions_and_debug_infos.is_empty()
    }
}
