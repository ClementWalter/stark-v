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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::VmOpcode;

    #[test]
    fn test_program_default() {
        let program = Program::default();
        assert!(program.is_empty());
        assert_eq!(program.len(), 0);
        assert_eq!(program.pc_base, 0);
    }

    #[test]
    fn test_program_from_instructions_empty() {
        let program = Program::from_instructions(vec![], 0x1000);
        assert!(program.is_empty());
        assert_eq!(program.len(), 0);
        assert_eq!(program.pc_base, 0x1000);
    }

    #[test]
    fn test_program_from_instructions_single() {
        let inst = Instruction::new(VmOpcode(0x200), 1, 2, 3, 4, 5, 6, 7);
        let program = Program::from_instructions(vec![inst.clone()], 0x2000);

        assert!(!program.is_empty());
        assert_eq!(program.len(), 1);
        assert_eq!(program.pc_base, 0x2000);

        let (stored_inst, debug_info) = program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap();
        assert_eq!(*stored_inst, inst);
        assert!(debug_info.is_none());
    }

    #[test]
    fn test_program_from_instructions_multiple() {
        let inst1 = Instruction::new(VmOpcode(0x200), 1, 0, 0, 0, 0, 0, 0);
        let inst2 = Instruction::new(VmOpcode(0x201), 2, 0, 0, 0, 0, 0, 0);
        let inst3 = Instruction::new(VmOpcode(0x202), 3, 0, 0, 0, 0, 0, 0);

        let program = Program::from_instructions(vec![inst1.clone(), inst2.clone(), inst3.clone()], 0x3000);

        assert_eq!(program.len(), 3);
        assert_eq!(program.pc_base, 0x3000);

        let (stored_inst1, _) = program.instructions_and_debug_infos[0].as_ref().unwrap();
        let (stored_inst2, _) = program.instructions_and_debug_infos[1].as_ref().unwrap();
        let (stored_inst3, _) = program.instructions_and_debug_infos[2].as_ref().unwrap();

        assert_eq!(*stored_inst1, inst1);
        assert_eq!(*stored_inst2, inst2);
        assert_eq!(*stored_inst3, inst3);
    }

    #[test]
    fn test_program_clone() {
        let inst = Instruction::new(VmOpcode(0x200), 1, 2, 3, 4, 5, 6, 7);
        let program1 = Program::from_instructions(vec![inst], 0x1000);
        let program2 = program1.clone();

        assert_eq!(program1.len(), program2.len());
        assert_eq!(program1.pc_base, program2.pc_base);
    }

    #[test]
    fn test_program_debug() {
        let program = Program::from_instructions(vec![], 0x1000);
        let debug_str = format!("{:?}", program);
        assert!(debug_str.contains("Program"));
        assert!(debug_str.contains("pc_base"));
    }
}
