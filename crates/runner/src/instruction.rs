#![allow(clippy::upper_case_acronyms)]
use std::fmt;

pub const RV32_REGISTER_NUM_LIMBS: usize = 4;
pub const RV32_MEMORY_AS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmOpcode(u32);

impl VmOpcode {
    pub const fn from_usize(value: usize) -> Self {
        Self(value as u32)
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for VmOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhantomDiscriminant(pub u16);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: VmOpcode,
    pub a: i64,
    pub b: i64,
    pub c: i64,
    pub d: i64,
    pub e: i64,
    pub f: i64,
    pub g: i64,
}

impl Instruction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(opcode: VmOpcode, a: i64, b: i64, c: i64, d: i64, e: i64, f: i64, g: i64) -> Self {
        Self {
            opcode,
            a,
            b,
            c,
            d,
            e,
            f,
            g,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_isize(opcode: VmOpcode, a: isize, b: isize, c: isize, d: isize, e: isize) -> Self {
        Self::new(
            opcode, a as i64, b as i64, c as i64, d as i64, e as i64, 0, 0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn large_from_isize(
        opcode: VmOpcode,
        a: isize,
        b: isize,
        c: isize,
        d: isize,
        e: isize,
        f: isize,
        g: isize,
    ) -> Self {
        Self::new(
            opcode, a as i64, b as i64, c as i64, d as i64, e as i64, f as i64, g as i64,
        )
    }

    pub fn phantom(discriminant: PhantomDiscriminant, a: i64, b: i64, c_upper: u16) -> Self {
        let c = (discriminant.0 as i64) | ((c_upper as i64) << 16);
        Self {
            opcode: SystemOpcode::PHANTOM.opcode(),
            a,
            b,
            c,
            d: 0,
            e: 0,
            f: 0,
            g: 0,
        }
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self {
            opcode: VmOpcode::from_usize(0),
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            g: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugInfo {
    pub dsl_instruction: String,
}

macro_rules! impl_local_opcode {
    ($name:ident) => {
        impl $name {
            pub const fn opcode(self) -> VmOpcode {
                VmOpcode::from_usize(self as usize)
            }
        }
    };
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemOpcode {
    TERMINATE = 0x0,
    PHANTOM = 0x1,
}
impl_local_opcode!(SystemOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseAluOpcode {
    ADD = 0x200,
    SUB = 0x201,
    XOR = 0x202,
    OR = 0x203,
    AND = 0x204,
}
impl_local_opcode!(BaseAluOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftOpcode {
    SLL = 0x205,
    SRL = 0x206,
    SRA = 0x207,
}
impl_local_opcode!(ShiftOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LessThanOpcode {
    SLT = 0x208,
    SLTU = 0x209,
}
impl_local_opcode!(LessThanOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32LoadStoreOpcode {
    LOADW = 0x210,
    LOADBU = 0x211,
    LOADHU = 0x212,
    STOREW = 0x213,
    STOREH = 0x214,
    STOREB = 0x215,
    LOADB = 0x216,
    LOADH = 0x217,
}
impl_local_opcode!(Rv32LoadStoreOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchEqualOpcode {
    BEQ = 0x220,
    BNE = 0x221,
}
impl_local_opcode!(BranchEqualOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchLessThanOpcode {
    BLT = 0x225,
    BLTU = 0x226,
    BGE = 0x227,
    BGEU = 0x228,
}
impl_local_opcode!(BranchLessThanOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32JalLuiOpcode {
    JAL = 0x230,
    LUI = 0x231,
}
impl_local_opcode!(Rv32JalLuiOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32JalrOpcode {
    JALR = 0x235,
}
impl_local_opcode!(Rv32JalrOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32AuipcOpcode {
    AUIPC = 0x240,
}
impl_local_opcode!(Rv32AuipcOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MulOpcode {
    MUL = 0x250,
}
impl_local_opcode!(MulOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MulHOpcode {
    MULH = 0x251,
    MULHSU = 0x252,
    MULHU = 0x253,
}
impl_local_opcode!(MulHOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivRemOpcode {
    DIV = 0x254,
    DIVU = 0x255,
    REM = 0x256,
    REMU = 0x257,
}
impl_local_opcode!(DivRemOpcode);

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Rv32HintStoreOpcode {
    HINT_STOREW = 0x260,
    HINT_BUFFER = 0x261,
}
impl_local_opcode!(Rv32HintStoreOpcode);

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32Phantom {
    HintInput = 0x20,
    PrintStr = 0x21,
    HintRandom = 0x22,
    HintLoadByKey = 0x23,
}
