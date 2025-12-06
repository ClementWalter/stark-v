use std::collections::BTreeMap;

use eyre::Result;
use rrs_lib::{
    instruction_formats::{BType, IType, ITypeShamt, JType, RType, SType, UType},
    process_instruction, InstructionProcessor,
};

use crate::{
    elf::Elf,
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
                    .map_err(|_| eyre::eyre!("TERMINATE imm must fit in u8"))?;
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
        _ => process_instruction(transpiler, word)
            .ok_or_else(|| eyre::eyre!("unsupported instruction: 0x{word:08x}")),
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
    if dec.rd == 0 {
        return nop();
    }
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
