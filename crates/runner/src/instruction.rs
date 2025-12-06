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

/// Unified macro for defining VM opcodes.
///
/// This macro generates:
/// - An enum with `#[repr(usize)]` representation
/// - Standard derives: Clone, Copy, Debug, PartialEq, Eq
/// - An `opcode()` method that converts the enum variant to a VmOpcode
///
/// Usage:
/// ```
/// define_opcodes! {
///     EnumName {
///         VARIANT1 = 0x100,
///         VARIANT2 = 0x101,
///     }
/// }
/// ```
///
/// For enums with non-camel-case variants, use the `@non_camel_case` attribute:
/// ```
/// define_opcodes! {
///     @non_camel_case
///     EnumName {
///         non_camel_case_variant = 0x100,
///     }
/// }
/// ```
macro_rules! define_opcodes {
    // Pattern for enums with non-camel-case variants
    (
        @non_camel_case
        $name:ident {
            $(
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        #[repr(usize)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[allow(non_camel_case_types)]
        pub enum $name {
            $(
                $variant = $value,
            )*
        }

        impl $name {
            pub const fn opcode(self) -> VmOpcode {
                VmOpcode::from_usize(self as usize)
            }
        }
    };

    // Standard pattern for enums
    (
        $name:ident {
            $(
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        #[repr(usize)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name {
            $(
                $variant = $value,
            )*
        }

        impl $name {
            pub const fn opcode(self) -> VmOpcode {
                VmOpcode::from_usize(self as usize)
            }
        }
    };
}

define_opcodes! {
    SystemOpcode {
        TERMINATE = 0x0,
        PHANTOM = 0x1,
    }
}

define_opcodes! {
    BaseAluOpcode {
        ADD = 0x200,
        SUB = 0x201,
        XOR = 0x202,
        OR = 0x203,
        AND = 0x204,
    }
}

define_opcodes! {
    ShiftOpcode {
        SLL = 0x205,
        SRL = 0x206,
        SRA = 0x207,
    }
}

define_opcodes! {
    LessThanOpcode {
        SLT = 0x208,
        SLTU = 0x209,
    }
}

define_opcodes! {
    Rv32LoadStoreOpcode {
        LOADW = 0x210,
        LOADBU = 0x211,
        LOADHU = 0x212,
        STOREW = 0x213,
        STOREH = 0x214,
        STOREB = 0x215,
        LOADB = 0x216,
        LOADH = 0x217,
    }
}

define_opcodes! {
    BranchEqualOpcode {
        BEQ = 0x220,
        BNE = 0x221,
    }
}

define_opcodes! {
    BranchLessThanOpcode {
        BLT = 0x225,
        BLTU = 0x226,
        BGE = 0x227,
        BGEU = 0x228,
    }
}

define_opcodes! {
    Rv32JalLuiOpcode {
        JAL = 0x230,
        LUI = 0x231,
    }
}

define_opcodes! {
    Rv32JalrOpcode {
        JALR = 0x235,
    }
}

define_opcodes! {
    Rv32AuipcOpcode {
        AUIPC = 0x240,
    }
}

define_opcodes! {
    MulOpcode {
        MUL = 0x250,
    }
}

define_opcodes! {
    MulHOpcode {
        MULH = 0x251,
        MULHSU = 0x252,
        MULHU = 0x253,
    }
}

define_opcodes! {
    DivRemOpcode {
        DIV = 0x254,
        DIVU = 0x255,
        REM = 0x256,
        REMU = 0x257,
    }
}

define_opcodes! {
    @non_camel_case
    Rv32HintStoreOpcode {
        HINT_STOREW = 0x260,
        HINT_BUFFER = 0x261,
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv32Phantom {
    HintInput = 0x20,
    PrintStr = 0x21,
    HintRandom = 0x22,
    HintLoadByKey = 0x23,
}
