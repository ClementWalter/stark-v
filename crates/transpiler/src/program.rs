use crate::instruction::Instruction;

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub pc_base: u32,
}

impl Program {
    pub fn from_instructions(instructions: Vec<Instruction>, pc_base: u32) -> Self {
        Self {
            instructions,
            pc_base,
        }
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
