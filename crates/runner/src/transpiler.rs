use std::collections::BTreeMap;

use rrs_lib::{
    instruction_formats::{BType, IType, ITypeShamt, JType, RType, SType, UType},
    process_instruction, InstructionProcessor,
};

use crate::{
    elf::Elf,
    error::{Result, RunnerError},
    instruction::{
        BaseAluOpcode, BranchEqualOpcode, BranchLessThanOpcode, DivRemOpcode, Instruction,
        LessThanOpcode, MulHOpcode, MulOpcode, PhantomDiscriminant, Rv32AuipcOpcode,
        Rv32HintStoreOpcode, Rv32JalLuiOpcode, Rv32JalrOpcode, Rv32LoadStoreOpcode, Rv32Phantom,
        ShiftOpcode, SystemOpcode, VmOpcode, RV32_MEMORY_AS, RV32_REGISTER_NUM_LIMBS,
    },
    program::Program,
    vm_exe::{SparseMemoryImage, VmExe},
};

const SYSTEM_OPCODE: u8 = 0x0b;
const CSR_OPCODE: u8 = 0b1_110_011;
const NATIVE_STOREW_FUNCT3: u8 = 0b111;
const NATIVE_STOREW_FUNCT7: u32 = 2;
const TERMINATE_FUNCT3: u8 = 0b000;
const HINT_FUNCT3: u8 = 0b001;
const HINT_STOREW_IMM: u32 = 0;
const HINT_BUFFER_IMM: u32 = 1;
const REVEAL_FUNCT3: u8 = 0b010;
const PHANTOM_FUNCT3: u8 = 0b011;
const CSRRW_FUNCT3: u8 = 0b001;

pub fn transpile_elf(elf: Elf) -> Result<VmExe> {
    let mut transpiler = InstructionTranspiler;
    let mut program_instructions = Vec::with_capacity(elf.instructions.len());
    for &word in &elf.instructions {
        let instruction = transpile_instruction(word, &mut transpiler)?;
        program_instructions.push(instruction);
    }
    let program = Program::from_instructions(program_instructions, elf.pc_base);
    let init_memory = elf_memory_image_to_vm_memory_image(elf.memory_image);
    Ok(VmExe::new(program, elf.pc_start, init_memory))
}

fn transpile_instruction(word: u32, transpiler: &mut InstructionTranspiler) -> Result<Instruction> {
    let opcode = (word & 0x7f) as u8;
    let funct3 = ((word >> 12) & 0b111) as u8;
    match opcode {
        CSR_OPCODE => {
            let dec = IType::new(word);
            if dec.funct3 as u8 == CSRRW_FUNCT3 && dec.rs1 == 0 && dec.rd == 0 {
                Ok(nop())
            } else {
                Ok(unimp())
            }
        }
        SYSTEM_OPCODE => match funct3 {
            TERMINATE_FUNCT3 => {
                let dec = IType::new(word);
                let exit_code: u8 = dec
                    .imm
                    .try_into()
                    .map_err(|_| RunnerError::TerminateImmTooBig)?;
                let instruction = Instruction {
                    opcode: SystemOpcode::TERMINATE.opcode(),
                    c: exit_code as i64,
                    ..Instruction::default()
                };
                Ok(instruction)
            }
            PHANTOM_FUNCT3 => {
                let dec = IType::new(word);
                let imm = (dec.imm as u32) & 0xffff;
                if let Some(phantom) = PhantomImm::from_repr(imm as u16) {
                    Ok(match phantom {
                        PhantomImm::HintInput => Instruction::phantom(
                            PhantomDiscriminant(Rv32Phantom::HintInput as u16),
                            0,
                            0,
                            0,
                        ),
                        PhantomImm::HintRandom => Instruction::phantom(
                            PhantomDiscriminant(Rv32Phantom::HintRandom as u16),
                            register_offset(dec.rd),
                            0,
                            0,
                        ),
                        PhantomImm::PrintStr => Instruction::phantom(
                            PhantomDiscriminant(Rv32Phantom::PrintStr as u16),
                            register_offset(dec.rd),
                            register_offset(dec.rs1),
                            0,
                        ),
                        PhantomImm::HintLoadByKey => Instruction::phantom(
                            PhantomDiscriminant(Rv32Phantom::HintLoadByKey as u16),
                            register_offset(dec.rd),
                            register_offset(dec.rs1),
                            0,
                        ),
                    })
                } else {
                    Ok(unimp())
                }
            }
            HINT_FUNCT3 => {
                let dec = IType::new(word);
                let imm = (dec.imm as u32) & 0xffff;
                let inst = match imm {
                    HINT_STOREW_IMM => Instruction::from_isize(
                        Rv32HintStoreOpcode::HINT_STOREW.opcode(),
                        0,
                        register_offset(dec.rd) as isize,
                        0,
                        1,
                        2,
                    ),
                    HINT_BUFFER_IMM => Instruction::from_isize(
                        Rv32HintStoreOpcode::HINT_BUFFER.opcode(),
                        register_offset(dec.rs1) as isize,
                        register_offset(dec.rd) as isize,
                        0,
                        1,
                        2,
                    ),
                    _ => nop(),
                };
                Ok(inst)
            }
            REVEAL_FUNCT3 => {
                let dec = IType::new(word);
                let imm = (dec.imm as u32) & 0xffff;
                Ok(Instruction::large_from_isize(
                    Rv32LoadStoreOpcode::STOREW.opcode(),
                    register_offset(dec.rs1) as isize,
                    register_offset(dec.rd) as isize,
                    imm as isize,
                    1,
                    3,
                    1,
                    (dec.imm < 0) as isize,
                ))
            }
            NATIVE_STOREW_FUNCT3 => {
                let dec = RType::new(word);
                if dec.funct7 != NATIVE_STOREW_FUNCT7 {
                    Ok(nop())
                } else {
                    Ok(Instruction::large_from_isize(
                        Rv32LoadStoreOpcode::STOREW.opcode(),
                        register_offset(dec.rs1) as isize,
                        register_offset(dec.rd) as isize,
                        0,
                        1,
                        4,
                        1,
                        0,
                    ))
                }
            }
            _ => Ok(nop()),
        },
        _ => process_instruction(transpiler, word).ok_or(RunnerError::UnsupportedInstruction(word)),
    }
}

struct InstructionTranspiler;

impl InstructionProcessor for InstructionTranspiler {
    type InstructionResult = Instruction;

    fn process_add(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(BaseAluOpcode::ADD.opcode(), 1, &dec, false)
    }

    fn process_addi(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(BaseAluOpcode::ADD.opcode(), &dec)
    }

    fn process_sub(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(BaseAluOpcode::SUB.opcode(), 1, &dec, false)
    }

    fn process_xor(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(BaseAluOpcode::XOR.opcode(), 1, &dec, false)
    }

    fn process_xori(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(BaseAluOpcode::XOR.opcode(), &dec)
    }

    fn process_or(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(BaseAluOpcode::OR.opcode(), 1, &dec, false)
    }

    fn process_ori(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(BaseAluOpcode::OR.opcode(), &dec)
    }

    fn process_and(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(BaseAluOpcode::AND.opcode(), 1, &dec, false)
    }

    fn process_andi(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(BaseAluOpcode::AND.opcode(), &dec)
    }

    fn process_sll(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(ShiftOpcode::SLL.opcode(), 1, &dec, false)
    }

    fn process_slli(&mut self, dec: ITypeShamt) -> Self::InstructionResult {
        from_i_type_shamt(ShiftOpcode::SLL.opcode(), &dec)
    }

    fn process_srl(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(ShiftOpcode::SRL.opcode(), 1, &dec, false)
    }

    fn process_srli(&mut self, dec: ITypeShamt) -> Self::InstructionResult {
        from_i_type_shamt(ShiftOpcode::SRL.opcode(), &dec)
    }

    fn process_sra(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(ShiftOpcode::SRA.opcode(), 1, &dec, false)
    }

    fn process_srai(&mut self, dec: ITypeShamt) -> Self::InstructionResult {
        from_i_type_shamt(ShiftOpcode::SRA.opcode(), &dec)
    }

    fn process_slt(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(LessThanOpcode::SLT.opcode(), 1, &dec, false)
    }

    fn process_slti(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(LessThanOpcode::SLT.opcode(), &dec)
    }

    fn process_sltu(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(LessThanOpcode::SLTU.opcode(), 1, &dec, false)
    }

    fn process_sltui(&mut self, dec: IType) -> Self::InstructionResult {
        from_i_type(LessThanOpcode::SLTU.opcode(), &dec)
    }

    fn process_lb(&mut self, dec: IType) -> Self::InstructionResult {
        from_load(Rv32LoadStoreOpcode::LOADB.opcode(), &dec)
    }

    fn process_lh(&mut self, dec: IType) -> Self::InstructionResult {
        from_load(Rv32LoadStoreOpcode::LOADH.opcode(), &dec)
    }

    fn process_lw(&mut self, dec: IType) -> Self::InstructionResult {
        from_load(Rv32LoadStoreOpcode::LOADW.opcode(), &dec)
    }

    fn process_lbu(&mut self, dec: IType) -> Self::InstructionResult {
        from_load(Rv32LoadStoreOpcode::LOADBU.opcode(), &dec)
    }

    fn process_lhu(&mut self, dec: IType) -> Self::InstructionResult {
        from_load(Rv32LoadStoreOpcode::LOADHU.opcode(), &dec)
    }

    fn process_sb(&mut self, dec: SType) -> Self::InstructionResult {
        from_s_type(Rv32LoadStoreOpcode::STOREB.opcode(), &dec)
    }

    fn process_sh(&mut self, dec: SType) -> Self::InstructionResult {
        from_s_type(Rv32LoadStoreOpcode::STOREH.opcode(), &dec)
    }

    fn process_sw(&mut self, dec: SType) -> Self::InstructionResult {
        from_s_type(Rv32LoadStoreOpcode::STOREW.opcode(), &dec)
    }

    fn process_beq(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchEqualOpcode::BEQ.opcode(), &dec)
    }

    fn process_bne(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchEqualOpcode::BNE.opcode(), &dec)
    }

    fn process_blt(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchLessThanOpcode::BLT.opcode(), &dec)
    }

    fn process_bge(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchLessThanOpcode::BGE.opcode(), &dec)
    }

    fn process_bltu(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchLessThanOpcode::BLTU.opcode(), &dec)
    }

    fn process_bgeu(&mut self, dec: BType) -> Self::InstructionResult {
        from_b_type(BranchLessThanOpcode::BGEU.opcode(), &dec)
    }

    fn process_jal(&mut self, dec: JType) -> Self::InstructionResult {
        from_j_type(Rv32JalLuiOpcode::JAL.opcode(), &dec)
    }

    fn process_jalr(&mut self, dec: IType) -> Self::InstructionResult {
        Instruction::new(
            Rv32JalrOpcode::JALR.opcode(),
            register_offset(dec.rd),
            register_offset(dec.rs1),
            ((dec.imm as u32) & 0xffff) as i64,
            1,
            0,
            (dec.rd != 0) as i64,
            (dec.imm < 0) as i64,
        )
    }

    fn process_lui(&mut self, dec: UType) -> Self::InstructionResult {
        if dec.rd == 0 {
            return nop();
        }
        let mut inst = from_u_type(Rv32JalLuiOpcode::LUI.opcode(), &dec);
        inst.f = 1;
        inst
    }

    fn process_auipc(&mut self, dec: UType) -> Self::InstructionResult {
        if dec.rd == 0 {
            return nop();
        }
        Instruction::new(
            Rv32AuipcOpcode::AUIPC.opcode(),
            register_offset(dec.rd),
            0,
            (((dec.imm as u32) & 0xfffff000) >> 8) as i64,
            1,
            0,
            0,
            0,
        )
    }

    fn process_mul(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(MulOpcode::MUL.opcode(), 0, &dec, false)
    }

    fn process_mulh(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(MulHOpcode::MULH.opcode(), 0, &dec, false)
    }

    fn process_mulhu(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(MulHOpcode::MULHU.opcode(), 0, &dec, false)
    }

    fn process_mulhsu(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(MulHOpcode::MULHSU.opcode(), 0, &dec, false)
    }

    fn process_div(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(DivRemOpcode::DIV.opcode(), 0, &dec, false)
    }

    fn process_divu(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(DivRemOpcode::DIVU.opcode(), 0, &dec, false)
    }

    fn process_rem(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(DivRemOpcode::REM.opcode(), 0, &dec, false)
    }

    fn process_remu(&mut self, dec: RType) -> Self::InstructionResult {
        from_r_type(DivRemOpcode::REMU.opcode(), 0, &dec, false)
    }

    fn process_fence(&mut self, _dec: IType) -> Self::InstructionResult {
        nop()
    }
}

fn register_offset(reg: usize) -> i64 {
    (RV32_REGISTER_NUM_LIMBS * reg) as i64
}

fn from_r_type(opcode: VmOpcode, e_as: i64, dec: &RType, allow_rd_zero: bool) -> Instruction {
    if !allow_rd_zero && dec.rd == 0 {
        return nop();
    }
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        register_offset(dec.rs1),
        register_offset(dec.rs2),
        1,
        e_as,
        0,
        0,
    )
}

fn from_i_type(opcode: VmOpcode, dec: &IType) -> Instruction {
    if dec.rd == 0 {
        return nop();
    }
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        register_offset(dec.rs1),
        i12_to_u24(dec.imm) as i64,
        1,
        0,
        0,
        0,
    )
}

fn from_load(opcode: VmOpcode, dec: &IType) -> Instruction {
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        register_offset(dec.rs1),
        ((dec.imm as u32) & 0xffff) as i64,
        1,
        2,
        (dec.rd != 0) as i64,
        (dec.imm < 0) as i64,
    )
}

fn from_i_type_shamt(opcode: VmOpcode, dec: &ITypeShamt) -> Instruction {
    if dec.rd == 0 {
        return nop();
    }
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        register_offset(dec.rs1),
        dec.shamt as i64,
        1,
        0,
        0,
        0,
    )
}

fn from_s_type(opcode: VmOpcode, dec: &SType) -> Instruction {
    Instruction::new(
        opcode,
        register_offset(dec.rs2),
        register_offset(dec.rs1),
        ((dec.imm as u32) & 0xffff) as i64,
        1,
        2,
        1,
        (dec.imm < 0) as i64,
    )
}

fn from_b_type(opcode: VmOpcode, dec: &BType) -> Instruction {
    Instruction::new(
        opcode,
        register_offset(dec.rs1),
        register_offset(dec.rs2),
        dec.imm as i64,
        1,
        1,
        0,
        0,
    )
}

fn from_j_type(opcode: VmOpcode, dec: &JType) -> Instruction {
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        0,
        dec.imm as i64,
        1,
        0,
        (dec.rd != 0) as i64,
        0,
    )
}

fn from_u_type(opcode: VmOpcode, dec: &UType) -> Instruction {
    // Note: callers should check rd == 0 before calling this function
    debug_assert!(dec.rd != 0, "rd == 0 should be handled by caller");
    Instruction::new(
        opcode,
        register_offset(dec.rd),
        0,
        (((dec.imm as u32) >> 12) & 0xfffff) as i64,
        1,
        0,
        0,
        0,
    )
}

fn i12_to_u24(imm: i32) -> u32 {
    (imm as u32) & 0x00ff_ffff
}

fn nop() -> Instruction {
    Instruction {
        opcode: SystemOpcode::PHANTOM.opcode(),
        ..Instruction::default()
    }
}

fn unimp() -> Instruction {
    Instruction {
        opcode: SystemOpcode::TERMINATE.opcode(),
        c: 2,
        ..Instruction::default()
    }
}

fn elf_memory_image_to_vm_memory_image(memory_image: BTreeMap<u32, u32>) -> SparseMemoryImage {
    let mut result = SparseMemoryImage::new();
    for (addr, word) in memory_image {
        for (i, byte) in word.to_le_bytes().into_iter().enumerate() {
            result.insert((RV32_MEMORY_AS, addr + i as u32), byte);
        }
    }
    result
}

// Use repr(u8) to ensure stable integer representation for instruction decoding.
// The enum discriminants correspond to immediate values in RISC-V phantom instructions.
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum PhantomImm {
    HintInput = 0,
    PrintStr = 1,
    HintRandom = 2,
    HintLoadByKey = 3,
}

impl PhantomImm {
    fn from_repr(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::HintInput),
            1 => Some(Self::PrintStr),
            2 => Some(Self::HintRandom),
            3 => Some(Self::HintLoadByKey),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    // Helper function to create an Elf for testing
    fn create_test_elf(instructions: Vec<u32>) -> Elf {
        let memory_image: BTreeMap<u32, u32> = instructions
            .iter()
            .enumerate()
            .map(|(i, &inst)| (0x1000 + (i as u32) * 4, inst))
            .collect();

        Elf {
            instructions,
            pc_start: 0x1000,
            pc_base: 0x1000,
            memory_image,
        }
    }

    // ========== R-Type ALU Instructions ==========

    #[test]
    fn test_transpile_add() {
        // ADD rd, rs1, rs2: opcode=0x33, funct3=0x0, funct7=0x00
        // ADD x1, x2, x3 (rd=1, rs1=2, rs2=3)
        let word: u32 = 0b0000000_00011_00010_000_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::ADD.opcode());
        assert_eq!(inst.a, 4); // rd * 4 = 1 * 4
        assert_eq!(inst.b, 8); // rs1 * 4 = 2 * 4
        assert_eq!(inst.c, 12); // rs2 * 4 = 3 * 4
    }

    #[test]
    fn test_transpile_add_rd_zero() {
        // ADD x0, x2, x3 - should become NOP
        let word: u32 = 0b0000000_00011_00010_000_00000_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_sub() {
        // SUB rd, rs1, rs2: opcode=0x33, funct3=0x0, funct7=0x20
        let word: u32 = 0b0100000_00011_00010_000_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::SUB.opcode());
    }

    #[test]
    fn test_transpile_xor() {
        // XOR rd, rs1, rs2: opcode=0x33, funct3=0x4, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_100_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::XOR.opcode());
    }

    #[test]
    fn test_transpile_or() {
        // OR rd, rs1, rs2: opcode=0x33, funct3=0x6, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_110_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::OR.opcode());
    }

    #[test]
    fn test_transpile_and() {
        // AND rd, rs1, rs2: opcode=0x33, funct3=0x7, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_111_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::AND.opcode());
    }

    // ========== I-Type ALU Instructions ==========

    #[test]
    fn test_transpile_addi() {
        // ADDI rd, rs1, imm: opcode=0x13, funct3=0x0
        // ADDI x1, x2, 42
        let word: u32 = 0b000000101010_00010_000_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::ADD.opcode());
        assert_eq!(inst.a, 4); // rd * 4
        assert_eq!(inst.b, 8); // rs1 * 4
    }

    #[test]
    fn test_transpile_addi_rd_zero() {
        // ADDI x0, x2, 42 - should become NOP
        let word: u32 = 0b000000101010_00010_000_00000_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_xori() {
        // XORI rd, rs1, imm: opcode=0x13, funct3=0x4
        let word: u32 = 0b000000000001_00010_100_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::XOR.opcode());
    }

    #[test]
    fn test_transpile_ori() {
        // ORI rd, rs1, imm: opcode=0x13, funct3=0x6
        let word: u32 = 0b000000000001_00010_110_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::OR.opcode());
    }

    #[test]
    fn test_transpile_andi() {
        // ANDI rd, rs1, imm: opcode=0x13, funct3=0x7
        let word: u32 = 0b000000000001_00010_111_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BaseAluOpcode::AND.opcode());
    }

    // ========== Shift Instructions ==========

    #[test]
    fn test_transpile_sll() {
        // SLL rd, rs1, rs2: opcode=0x33, funct3=0x1, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_001_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SLL.opcode());
    }

    #[test]
    fn test_transpile_srl() {
        // SRL rd, rs1, rs2: opcode=0x33, funct3=0x5, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_101_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SRL.opcode());
    }

    #[test]
    fn test_transpile_sra() {
        // SRA rd, rs1, rs2: opcode=0x33, funct3=0x5, funct7=0x20
        let word: u32 = 0b0100000_00011_00010_101_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SRA.opcode());
    }

    #[test]
    fn test_transpile_slli() {
        // SLLI rd, rs1, shamt: opcode=0x13, funct3=0x1
        let word: u32 = 0b0000000_00101_00010_001_00001_0010011; // shamt=5
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SLL.opcode());
        assert_eq!(inst.c, 5); // shamt
    }

    #[test]
    fn test_transpile_srli() {
        // SRLI rd, rs1, shamt: opcode=0x13, funct3=0x5, funct7=0x00
        let word: u32 = 0b0000000_00101_00010_101_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SRL.opcode());
    }

    #[test]
    fn test_transpile_srai() {
        // SRAI rd, rs1, shamt: opcode=0x13, funct3=0x5, funct7=0x20
        let word: u32 = 0b0100000_00101_00010_101_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, ShiftOpcode::SRA.opcode());
    }

    // ========== Compare Instructions ==========

    #[test]
    fn test_transpile_slt() {
        // SLT rd, rs1, rs2: opcode=0x33, funct3=0x2, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_010_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, LessThanOpcode::SLT.opcode());
    }

    #[test]
    fn test_transpile_sltu() {
        // SLTU rd, rs1, rs2: opcode=0x33, funct3=0x3, funct7=0x00
        let word: u32 = 0b0000000_00011_00010_011_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, LessThanOpcode::SLTU.opcode());
    }

    #[test]
    fn test_transpile_slti() {
        // SLTI rd, rs1, imm: opcode=0x13, funct3=0x2
        let word: u32 = 0b000000000001_00010_010_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, LessThanOpcode::SLT.opcode());
    }

    #[test]
    fn test_transpile_sltiu() {
        // SLTIU rd, rs1, imm: opcode=0x13, funct3=0x3
        let word: u32 = 0b000000000001_00010_011_00001_0010011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, LessThanOpcode::SLTU.opcode());
    }

    // ========== Load Instructions ==========

    #[test]
    fn test_transpile_lb() {
        // LB rd, offset(rs1): opcode=0x03, funct3=0x0
        let word: u32 = 0b000000000100_00010_000_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADB.opcode());
    }

    #[test]
    fn test_transpile_lh() {
        // LH rd, offset(rs1): opcode=0x03, funct3=0x1
        let word: u32 = 0b000000000100_00010_001_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADH.opcode());
    }

    #[test]
    fn test_transpile_lw() {
        // LW rd, offset(rs1): opcode=0x03, funct3=0x2
        let word: u32 = 0b000000000100_00010_010_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADW.opcode());
    }

    #[test]
    fn test_transpile_lbu() {
        // LBU rd, offset(rs1): opcode=0x03, funct3=0x4
        let word: u32 = 0b000000000100_00010_100_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADBU.opcode());
    }

    #[test]
    fn test_transpile_lhu() {
        // LHU rd, offset(rs1): opcode=0x03, funct3=0x5
        let word: u32 = 0b000000000100_00010_101_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADHU.opcode());
    }

    #[test]
    fn test_transpile_load_negative_offset() {
        // LW rd, -4(rs1) - test negative immediate handling
        let word: u32 = 0b111111111100_00010_010_00001_0000011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::LOADW.opcode());
        assert_eq!(inst.g, 1); // negative flag
    }

    // ========== Store Instructions ==========

    #[test]
    fn test_transpile_sb() {
        // SB rs2, offset(rs1): opcode=0x23, funct3=0x0
        let word: u32 = 0b0000000_00011_00010_000_00100_0100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREB.opcode());
    }

    #[test]
    fn test_transpile_sh() {
        // SH rs2, offset(rs1): opcode=0x23, funct3=0x1
        let word: u32 = 0b0000000_00011_00010_001_00100_0100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREH.opcode());
    }

    #[test]
    fn test_transpile_sw() {
        // SW rs2, offset(rs1): opcode=0x23, funct3=0x2
        let word: u32 = 0b0000000_00011_00010_010_00100_0100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREW.opcode());
    }

    // ========== Branch Instructions ==========

    #[test]
    fn test_transpile_beq() {
        // BEQ rs1, rs2, offset: opcode=0x63, funct3=0x0
        let word: u32 = 0b0000000_00011_00010_000_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchEqualOpcode::BEQ.opcode());
    }

    #[test]
    fn test_transpile_bne() {
        // BNE rs1, rs2, offset: opcode=0x63, funct3=0x1
        let word: u32 = 0b0000000_00011_00010_001_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchEqualOpcode::BNE.opcode());
    }

    #[test]
    fn test_transpile_blt() {
        // BLT rs1, rs2, offset: opcode=0x63, funct3=0x4
        let word: u32 = 0b0000000_00011_00010_100_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchLessThanOpcode::BLT.opcode());
    }

    #[test]
    fn test_transpile_bge() {
        // BGE rs1, rs2, offset: opcode=0x63, funct3=0x5
        let word: u32 = 0b0000000_00011_00010_101_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchLessThanOpcode::BGE.opcode());
    }

    #[test]
    fn test_transpile_bltu() {
        // BLTU rs1, rs2, offset: opcode=0x63, funct3=0x6
        let word: u32 = 0b0000000_00011_00010_110_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchLessThanOpcode::BLTU.opcode());
    }

    #[test]
    fn test_transpile_bgeu() {
        // BGEU rs1, rs2, offset: opcode=0x63, funct3=0x7
        let word: u32 = 0b0000000_00011_00010_111_00100_1100011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, BranchLessThanOpcode::BGEU.opcode());
    }

    // ========== Jump Instructions ==========

    #[test]
    fn test_transpile_jal() {
        // JAL rd, offset: opcode=0x6F
        // JAL x1, 0x100
        let word: u32 = 0b0_0000001000_0_00000000_00001_1101111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32JalLuiOpcode::JAL.opcode());
        assert_eq!(inst.a, 4); // rd * 4
        assert_eq!(inst.f, 1); // rd != 0
    }

    #[test]
    fn test_transpile_jal_rd_zero() {
        // JAL x0, offset (J pseudo-instruction)
        let word: u32 = 0b0_0000001000_0_00000000_00000_1101111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32JalLuiOpcode::JAL.opcode());
        assert_eq!(inst.f, 0); // rd == 0
    }

    #[test]
    fn test_transpile_jalr() {
        // JALR rd, rs1, offset: opcode=0x67, funct3=0x0
        let word: u32 = 0b000000000100_00010_000_00001_1100111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32JalrOpcode::JALR.opcode());
        assert_eq!(inst.a, 4); // rd * 4
        assert_eq!(inst.b, 8); // rs1 * 4
        assert_eq!(inst.f, 1); // rd != 0
    }

    #[test]
    fn test_transpile_jalr_negative_offset() {
        // JALR rd, rs1, -4
        let word: u32 = 0b111111111100_00010_000_00001_1100111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32JalrOpcode::JALR.opcode());
        assert_eq!(inst.g, 1); // negative flag
    }

    // ========== Upper Immediate Instructions ==========

    #[test]
    fn test_transpile_lui() {
        // LUI rd, imm: opcode=0x37
        let word: u32 = 0b00000000000000010000_00001_0110111; // LUI x1, 0x10
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32JalLuiOpcode::LUI.opcode());
        assert_eq!(inst.f, 1);
    }

    #[test]
    fn test_transpile_lui_rd_zero() {
        // LUI x0, imm - should become NOP
        let word: u32 = 0b00000000000000010000_00000_0110111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_auipc() {
        // AUIPC rd, imm: opcode=0x17
        let word: u32 = 0b00000000000000010000_00001_0010111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32AuipcOpcode::AUIPC.opcode());
    }

    #[test]
    fn test_transpile_auipc_rd_zero() {
        // AUIPC x0, imm - should become NOP
        let word: u32 = 0b00000000000000010000_00000_0010111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    // ========== M Extension (Multiply/Divide) ==========

    #[test]
    fn test_transpile_mul() {
        // MUL rd, rs1, rs2: opcode=0x33, funct3=0x0, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_000_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, MulOpcode::MUL.opcode());
    }

    #[test]
    fn test_transpile_mulh() {
        // MULH rd, rs1, rs2: opcode=0x33, funct3=0x1, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_001_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, MulHOpcode::MULH.opcode());
    }

    #[test]
    fn test_transpile_mulhsu() {
        // MULHSU rd, rs1, rs2: opcode=0x33, funct3=0x2, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_010_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, MulHOpcode::MULHSU.opcode());
    }

    #[test]
    fn test_transpile_mulhu() {
        // MULHU rd, rs1, rs2: opcode=0x33, funct3=0x3, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_011_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, MulHOpcode::MULHU.opcode());
    }

    #[test]
    fn test_transpile_div() {
        // DIV rd, rs1, rs2: opcode=0x33, funct3=0x4, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_100_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, DivRemOpcode::DIV.opcode());
    }

    #[test]
    fn test_transpile_divu() {
        // DIVU rd, rs1, rs2: opcode=0x33, funct3=0x5, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_101_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, DivRemOpcode::DIVU.opcode());
    }

    #[test]
    fn test_transpile_rem() {
        // REM rd, rs1, rs2: opcode=0x33, funct3=0x6, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_110_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, DivRemOpcode::REM.opcode());
    }

    #[test]
    fn test_transpile_remu() {
        // REMU rd, rs1, rs2: opcode=0x33, funct3=0x7, funct7=0x01
        let word: u32 = 0b0000001_00011_00010_111_00001_0110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, DivRemOpcode::REMU.opcode());
    }

    // ========== System Instructions ==========

    #[test]
    fn test_transpile_terminate() {
        // TERMINATE with exit code 0
        // System opcode: 0x0b, funct3=0x0
        let word: u32 = 0b000000000000_00000_000_00000_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::TERMINATE.opcode());
        assert_eq!(inst.c, 0); // exit code
    }

    #[test]
    fn test_transpile_terminate_with_code() {
        // TERMINATE with exit code 42
        let word: u32 = 0b000000101010_00000_000_00000_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::TERMINATE.opcode());
        assert_eq!(inst.c, 42);
    }

    #[test]
    fn test_transpile_terminate_imm_too_big() {
        // TERMINATE with imm > 255 should fail
        let word: u32 = 0b000100000000_00000_000_00000_0001011; // imm=256
        let elf = create_test_elf(vec![word]);
        let result = transpile_elf(elf);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::TerminateImmTooBig
        ));
    }

    #[test]
    fn test_transpile_phantom_hint_input() {
        // PHANTOM with imm=0 (HintInput)
        let word: u32 = 0b000000000000_00000_011_00000_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
    }

    #[test]
    fn test_transpile_phantom_print_str() {
        // PHANTOM with imm=1 (PrintStr)
        let word: u32 = 0b000000000001_00001_011_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
    }

    #[test]
    fn test_transpile_phantom_hint_random() {
        // PHANTOM with imm=2 (HintRandom)
        let word: u32 = 0b000000000010_00000_011_00001_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
    }

    #[test]
    fn test_transpile_phantom_hint_load_by_key() {
        // PHANTOM with imm=3 (HintLoadByKey)
        let word: u32 = 0b000000000011_00001_011_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
    }

    #[test]
    fn test_transpile_phantom_unknown() {
        // PHANTOM with unknown imm should become TERMINATE
        let word: u32 = 0b000011111111_00000_011_00000_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::TERMINATE.opcode()); // unimp
    }

    #[test]
    fn test_transpile_hint_storew() {
        // HINT_STOREW: funct3=0x1, imm=0
        let word: u32 = 0b000000000000_00001_001_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32HintStoreOpcode::HINT_STOREW.opcode());
    }

    #[test]
    fn test_transpile_hint_buffer() {
        // HINT_BUFFER: funct3=0x1, imm=1
        let word: u32 = 0b000000000001_00001_001_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32HintStoreOpcode::HINT_BUFFER.opcode());
    }

    #[test]
    fn test_transpile_hint_unknown_becomes_nop() {
        // HINT with unknown imm becomes NOP
        let word: u32 = 0b000000000010_00001_001_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_reveal() {
        // REVEAL: funct3=0x2
        let word: u32 = 0b000000000100_00001_010_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREW.opcode());
    }

    #[test]
    fn test_transpile_reveal_negative() {
        // REVEAL with negative immediate
        let word: u32 = 0b111111111100_00001_010_00010_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREW.opcode());
        assert_eq!(inst.g, 1); // negative flag
    }

    #[test]
    fn test_transpile_native_storew() {
        // NATIVE_STOREW: funct3=0x7, funct7=2
        let word: u32 = 0b0000010_00011_00010_111_00001_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, Rv32LoadStoreOpcode::STOREW.opcode());
    }

    #[test]
    fn test_transpile_native_storew_wrong_funct7() {
        // NATIVE_STOREW with wrong funct7 becomes NOP
        let word: u32 = 0b0000011_00011_00010_111_00001_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_system_unknown_funct3() {
        // System opcode with unknown funct3 becomes NOP
        let word: u32 = 0b000000000000_00000_100_00000_0001011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    // ========== CSR Instructions ==========

    #[test]
    fn test_transpile_csrrw_nop() {
        // CSRRW x0, csr, x0 - becomes NOP
        let word: u32 = 0b000000000000_00000_001_00000_1110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_csrrw_not_nop() {
        // CSRRW with non-zero rs1 or rd becomes UNIMP (TERMINATE)
        let word: u32 = 0b000000000000_00001_001_00000_1110011;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::TERMINATE.opcode()); // UNIMP
    }

    // ========== FENCE Instruction ==========

    #[test]
    fn test_transpile_fence() {
        // FENCE: opcode=0x0F
        let word: u32 = 0b0000_0000_0000_00000_000_00000_0001111;
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    // ========== Unsupported Instructions ==========

    #[test]
    fn test_transpile_unsupported_instruction() {
        // Completely invalid opcode
        let word: u32 = 0xFFFFFFFF;
        let elf = create_test_elf(vec![word]);
        let result = transpile_elf(elf);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::UnsupportedInstruction(_)
        ));
    }

    // ========== Memory Image Conversion ==========

    #[test]
    fn test_elf_memory_image_to_vm_memory_image() {
        let mut memory_image = BTreeMap::new();
        memory_image.insert(0x1000, 0xDEADBEEF);
        memory_image.insert(0x1004, 0x12345678);

        let vm_mem = elf_memory_image_to_vm_memory_image(memory_image);

        // Check first word bytes
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1000)), Some(&0xEF));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1001)), Some(&0xBE));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1002)), Some(&0xAD));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1003)), Some(&0xDE));

        // Check second word bytes
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1004)), Some(&0x78));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1005)), Some(&0x56));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1006)), Some(&0x34));
        assert_eq!(vm_mem.get(&(RV32_MEMORY_AS, 0x1007)), Some(&0x12));
    }

    #[test]
    fn test_elf_memory_image_to_vm_memory_image_empty() {
        let memory_image = BTreeMap::new();
        let vm_mem = elf_memory_image_to_vm_memory_image(memory_image);
        assert!(vm_mem.is_empty());
    }

    // ========== Multiple Instructions ==========

    #[test]
    fn test_transpile_multiple_instructions() {
        let instructions = vec![
            0b0000000_00011_00010_000_00001_0110011, // ADD x1, x2, x3
            0b0100000_00011_00010_000_00001_0110011, // SUB x1, x2, x3
            0b0000000_00011_00010_100_00001_0110011, // XOR x1, x2, x3
        ];
        let elf = create_test_elf(instructions);
        let vm_exe = transpile_elf(elf).unwrap();

        assert_eq!(vm_exe.program.len(), 3);
        assert_eq!(
            vm_exe.program.instructions_and_debug_infos[0]
                .as_ref()
                .unwrap()
                .0
                .opcode,
            BaseAluOpcode::ADD.opcode()
        );
        assert_eq!(
            vm_exe.program.instructions_and_debug_infos[1]
                .as_ref()
                .unwrap()
                .0
                .opcode,
            BaseAluOpcode::SUB.opcode()
        );
        assert_eq!(
            vm_exe.program.instructions_and_debug_infos[2]
                .as_ref()
                .unwrap()
                .0
                .opcode,
            BaseAluOpcode::XOR.opcode()
        );
    }

    #[test]
    fn test_transpile_elf_preserves_pc_start() {
        let mut elf = create_test_elf(vec![0b0000000_00011_00010_000_00001_0110011]);
        elf.pc_start = 0x2000;
        let vm_exe = transpile_elf(elf).unwrap();
        assert_eq!(vm_exe.pc_start, 0x2000);
    }

    // ========== Helper Functions ==========

    #[test]
    fn test_i12_to_u24() {
        assert_eq!(i12_to_u24(0), 0);
        assert_eq!(i12_to_u24(42), 42);
        assert_eq!(i12_to_u24(-1), 0x00FFFFFF);
        assert_eq!(i12_to_u24(-4), 0x00FFFFFC);
    }

    #[test]
    fn test_register_offset() {
        assert_eq!(register_offset(0), 0);
        assert_eq!(register_offset(1), 4);
        assert_eq!(register_offset(31), 124);
    }

    #[test]
    fn test_nop() {
        let inst = nop();
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
        assert_eq!(inst.a, 0);
        assert_eq!(inst.b, 0);
        assert_eq!(inst.c, 0);
    }

    #[test]
    fn test_unimp() {
        let inst = unimp();
        assert_eq!(inst.opcode, SystemOpcode::TERMINATE.opcode());
        assert_eq!(inst.c, 2);
    }

    #[test]
    fn test_phantom_imm_from_repr() {
        assert!(matches!(
            PhantomImm::from_repr(0),
            Some(PhantomImm::HintInput)
        ));
        assert!(matches!(
            PhantomImm::from_repr(1),
            Some(PhantomImm::PrintStr)
        ));
        assert!(matches!(
            PhantomImm::from_repr(2),
            Some(PhantomImm::HintRandom)
        ));
        assert!(matches!(
            PhantomImm::from_repr(3),
            Some(PhantomImm::HintLoadByKey)
        ));
        assert!(PhantomImm::from_repr(4).is_none());
        assert!(PhantomImm::from_repr(255).is_none());
    }

    #[test]
    fn test_transpile_slli_rd_zero() {
        // SLLI x0, rs1, shamt: should become NOP
        let word: u32 = 0b0000000_00101_00010_001_00000_0010011; // rd=0, shamt=5
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_srli_rd_zero() {
        // SRLI x0, rs1, shamt: should become NOP
        let word: u32 = 0b0000000_00101_00010_101_00000_0010011; // rd=0
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }

    #[test]
    fn test_transpile_srai_rd_zero() {
        // SRAI x0, rs1, shamt: should become NOP
        let word: u32 = 0b0100000_00101_00010_101_00000_0010011; // rd=0
        let elf = create_test_elf(vec![word]);
        let vm_exe = transpile_elf(elf).unwrap();
        let inst = &vm_exe.program.instructions_and_debug_infos[0]
            .as_ref()
            .unwrap()
            .0;
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode()); // NOP
    }
}
