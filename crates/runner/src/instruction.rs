#![allow(clippy::upper_case_acronyms)]
use derive_more::Display;

pub const RV32_REGISTER_NUM_LIMBS: usize = 4;
pub const RV32_MEMORY_AS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
#[display("{_0}")]
pub struct VmOpcode(pub u32);

impl VmOpcode {
    pub const fn from_usize(value: usize) -> Self {
        Self(value as u32)
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
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
///     SystemOpcode {
///         TERMINATE = 0x0,
///         PHANTOM = 0x1,
///     },
///     BaseAluOpcode {
///         ADD = 0x200,
///         SUB = 0x201,
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
    // Entry point: process all enums sequentially
    (
        @non_camel_case
        $name:ident {
            $(
                $variant:ident = $value:expr
            ),* $(,)?
        }
        $(,
            $(@ $rest_attr:ident)?
            $rest_name:ident {
                $(
                    $rest_variant:ident = $rest_value:expr
                ),* $(,)?
            }
        )*
        $(,)?
    ) => {
        define_opcodes! {
            @single
            @non_camel_case
            $name {
                $(
                    $variant = $value
                ),*
            }
        }

        $(
            define_opcodes! {
                $(@ $rest_attr)?
                $rest_name {
                    $(
                        $rest_variant = $rest_value
                    ),*
                }
            }
        )*
    };

    // Entry point: standard enum followed by more enums
    (
        $name:ident {
            $(
                $variant:ident = $value:expr
            ),* $(,)?
        }
        $(,
            $(@ $rest_attr:ident)?
            $rest_name:ident {
                $(
                    $rest_variant:ident = $rest_value:expr
                ),* $(,)?
            }
        )*
        $(,)?
    ) => {
        define_opcodes! {
            @single
            $name {
                $(
                    $variant = $value
                ),*
            }
        }

        $(
            define_opcodes! {
                $(@ $rest_attr)?
                $rest_name {
                    $(
                        $rest_variant = $rest_value
                    ),*
                }
            }
        )*
    };

    // Internal: single enum with non-camel-case variants
    (
        @single
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

    // Internal: standard single enum
    (
        @single
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
    },
    BaseAluOpcode {
        ADD = 0x200,
        SUB = 0x201,
        XOR = 0x202,
        OR = 0x203,
        AND = 0x204,
    },
    ShiftOpcode {
        SLL = 0x205,
        SRL = 0x206,
        SRA = 0x207,
    },
    LessThanOpcode {
        SLT = 0x208,
        SLTU = 0x209,
    },
    Rv32LoadStoreOpcode {
        LOADW = 0x210,
        LOADBU = 0x211,
        LOADHU = 0x212,
        STOREW = 0x213,
        STOREH = 0x214,
        STOREB = 0x215,
        LOADB = 0x216,
        LOADH = 0x217,
    },
    BranchEqualOpcode {
        BEQ = 0x220,
        BNE = 0x221,
    },
    BranchLessThanOpcode {
        BLT = 0x225,
        BLTU = 0x226,
        BGE = 0x227,
        BGEU = 0x228,
    },
    Rv32JalLuiOpcode {
        JAL = 0x230,
        LUI = 0x231,
    },
    Rv32JalrOpcode {
        JALR = 0x235,
    },
    Rv32AuipcOpcode {
        AUIPC = 0x240,
    },
    MulOpcode {
        MUL = 0x250,
    },
    MulHOpcode {
        MULH = 0x251,
        MULHSU = 0x252,
        MULHU = 0x253,
    },
    DivRemOpcode {
        DIV = 0x254,
        DIVU = 0x255,
        REM = 0x256,
        REMU = 0x257,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_opcode_from_usize() {
        let opcode = VmOpcode::from_usize(0x200);
        assert_eq!(opcode.0, 0x200);
    }

    #[test]
    fn test_vm_opcode_as_usize() {
        let opcode = VmOpcode(0x250);
        assert_eq!(opcode.as_usize(), 0x250);
    }

    #[test]
    fn test_vm_opcode_display() {
        let opcode = VmOpcode(42);
        assert_eq!(format!("{}", opcode), "42");
    }

    #[test]
    fn test_vm_opcode_equality() {
        let op1 = VmOpcode(100);
        let op2 = VmOpcode(100);
        let op3 = VmOpcode(200);
        assert_eq!(op1, op2);
        assert_ne!(op1, op3);
    }

    #[test]
    fn test_vm_opcode_clone_copy() {
        let op1 = VmOpcode(100);
        let op2 = op1; // Copy
        assert_eq!(op1, op2);
    }

    #[test]
    fn test_phantom_discriminant_default() {
        let pd = PhantomDiscriminant::default();
        assert_eq!(pd.0, 0);
    }

    #[test]
    fn test_phantom_discriminant_equality() {
        let pd1 = PhantomDiscriminant(10);
        let pd2 = PhantomDiscriminant(10);
        let pd3 = PhantomDiscriminant(20);
        assert_eq!(pd1, pd2);
        assert_ne!(pd1, pd3);
    }

    #[test]
    fn test_instruction_new() {
        let inst = Instruction::new(VmOpcode(0x200), 1, 2, 3, 4, 5, 6, 7);
        assert_eq!(inst.opcode.0, 0x200);
        assert_eq!(inst.a, 1);
        assert_eq!(inst.b, 2);
        assert_eq!(inst.c, 3);
        assert_eq!(inst.d, 4);
        assert_eq!(inst.e, 5);
        assert_eq!(inst.f, 6);
        assert_eq!(inst.g, 7);
    }

    #[test]
    fn test_instruction_from_isize() {
        let inst = Instruction::from_isize(VmOpcode(0x201), 10, 20, 30, 40, 50);
        assert_eq!(inst.opcode.0, 0x201);
        assert_eq!(inst.a, 10);
        assert_eq!(inst.b, 20);
        assert_eq!(inst.c, 30);
        assert_eq!(inst.d, 40);
        assert_eq!(inst.e, 50);
        assert_eq!(inst.f, 0);
        assert_eq!(inst.g, 0);
    }

    #[test]
    fn test_instruction_large_from_isize() {
        let inst = Instruction::large_from_isize(VmOpcode(0x202), 1, 2, 3, 4, 5, 6, 7);
        assert_eq!(inst.opcode.0, 0x202);
        assert_eq!(inst.a, 1);
        assert_eq!(inst.b, 2);
        assert_eq!(inst.c, 3);
        assert_eq!(inst.d, 4);
        assert_eq!(inst.e, 5);
        assert_eq!(inst.f, 6);
        assert_eq!(inst.g, 7);
    }

    #[test]
    fn test_instruction_phantom() {
        let inst = Instruction::phantom(PhantomDiscriminant(0x20), 100, 200, 0x1234);
        assert_eq!(inst.opcode, SystemOpcode::PHANTOM.opcode());
        assert_eq!(inst.a, 100);
        assert_eq!(inst.b, 200);
        // c = discriminant | (c_upper << 16)
        assert_eq!(inst.c, 0x20 | (0x1234i64 << 16));
        assert_eq!(inst.d, 0);
        assert_eq!(inst.e, 0);
        assert_eq!(inst.f, 0);
        assert_eq!(inst.g, 0);
    }

    #[test]
    fn test_instruction_default() {
        let inst = Instruction::default();
        assert_eq!(inst.opcode, VmOpcode::from_usize(0));
        assert_eq!(inst.a, 0);
        assert_eq!(inst.b, 0);
        assert_eq!(inst.c, 0);
        assert_eq!(inst.d, 0);
        assert_eq!(inst.e, 0);
        assert_eq!(inst.f, 0);
        assert_eq!(inst.g, 0);
    }

    #[test]
    fn test_instruction_equality() {
        let inst1 = Instruction::new(VmOpcode(100), 1, 2, 3, 4, 5, 6, 7);
        let inst2 = Instruction::new(VmOpcode(100), 1, 2, 3, 4, 5, 6, 7);
        let inst3 = Instruction::new(VmOpcode(100), 1, 2, 3, 4, 5, 6, 8);
        assert_eq!(inst1, inst2);
        assert_ne!(inst1, inst3);
    }

    #[test]
    fn test_instruction_clone() {
        let inst1 = Instruction::new(VmOpcode(200), 10, 20, 30, 40, 50, 60, 70);
        let inst2 = inst1.clone();
        assert_eq!(inst1, inst2);
    }

    #[test]
    fn test_debug_info_default() {
        let di = DebugInfo::default();
        assert_eq!(di.dsl_instruction, "");
    }

    #[test]
    fn test_debug_info_equality() {
        let di1 = DebugInfo {
            dsl_instruction: "ADD".to_string(),
        };
        let di2 = DebugInfo {
            dsl_instruction: "ADD".to_string(),
        };
        let di3 = DebugInfo {
            dsl_instruction: "SUB".to_string(),
        };
        assert_eq!(di1, di2);
        assert_ne!(di1, di3);
    }

    // Test opcodes
    #[test]
    fn test_system_opcodes() {
        assert_eq!(SystemOpcode::TERMINATE.opcode().0, 0x0);
        assert_eq!(SystemOpcode::PHANTOM.opcode().0, 0x1);
    }

    #[test]
    fn test_base_alu_opcodes() {
        assert_eq!(BaseAluOpcode::ADD.opcode().0, 0x200);
        assert_eq!(BaseAluOpcode::SUB.opcode().0, 0x201);
        assert_eq!(BaseAluOpcode::XOR.opcode().0, 0x202);
        assert_eq!(BaseAluOpcode::OR.opcode().0, 0x203);
        assert_eq!(BaseAluOpcode::AND.opcode().0, 0x204);
    }

    #[test]
    fn test_shift_opcodes() {
        assert_eq!(ShiftOpcode::SLL.opcode().0, 0x205);
        assert_eq!(ShiftOpcode::SRL.opcode().0, 0x206);
        assert_eq!(ShiftOpcode::SRA.opcode().0, 0x207);
    }

    #[test]
    fn test_less_than_opcodes() {
        assert_eq!(LessThanOpcode::SLT.opcode().0, 0x208);
        assert_eq!(LessThanOpcode::SLTU.opcode().0, 0x209);
    }

    #[test]
    fn test_load_store_opcodes() {
        assert_eq!(Rv32LoadStoreOpcode::LOADW.opcode().0, 0x210);
        assert_eq!(Rv32LoadStoreOpcode::LOADBU.opcode().0, 0x211);
        assert_eq!(Rv32LoadStoreOpcode::LOADHU.opcode().0, 0x212);
        assert_eq!(Rv32LoadStoreOpcode::STOREW.opcode().0, 0x213);
        assert_eq!(Rv32LoadStoreOpcode::STOREH.opcode().0, 0x214);
        assert_eq!(Rv32LoadStoreOpcode::STOREB.opcode().0, 0x215);
        assert_eq!(Rv32LoadStoreOpcode::LOADB.opcode().0, 0x216);
        assert_eq!(Rv32LoadStoreOpcode::LOADH.opcode().0, 0x217);
    }

    #[test]
    fn test_branch_equal_opcodes() {
        assert_eq!(BranchEqualOpcode::BEQ.opcode().0, 0x220);
        assert_eq!(BranchEqualOpcode::BNE.opcode().0, 0x221);
    }

    #[test]
    fn test_branch_less_than_opcodes() {
        assert_eq!(BranchLessThanOpcode::BLT.opcode().0, 0x225);
        assert_eq!(BranchLessThanOpcode::BLTU.opcode().0, 0x226);
        assert_eq!(BranchLessThanOpcode::BGE.opcode().0, 0x227);
        assert_eq!(BranchLessThanOpcode::BGEU.opcode().0, 0x228);
    }

    #[test]
    fn test_jal_lui_opcodes() {
        assert_eq!(Rv32JalLuiOpcode::JAL.opcode().0, 0x230);
        assert_eq!(Rv32JalLuiOpcode::LUI.opcode().0, 0x231);
    }

    #[test]
    fn test_jalr_opcode() {
        assert_eq!(Rv32JalrOpcode::JALR.opcode().0, 0x235);
    }

    #[test]
    fn test_auipc_opcode() {
        assert_eq!(Rv32AuipcOpcode::AUIPC.opcode().0, 0x240);
    }

    #[test]
    fn test_mul_opcodes() {
        assert_eq!(MulOpcode::MUL.opcode().0, 0x250);
        assert_eq!(MulHOpcode::MULH.opcode().0, 0x251);
        assert_eq!(MulHOpcode::MULHSU.opcode().0, 0x252);
        assert_eq!(MulHOpcode::MULHU.opcode().0, 0x253);
    }

    #[test]
    fn test_div_rem_opcodes() {
        assert_eq!(DivRemOpcode::DIV.opcode().0, 0x254);
        assert_eq!(DivRemOpcode::DIVU.opcode().0, 0x255);
        assert_eq!(DivRemOpcode::REM.opcode().0, 0x256);
        assert_eq!(DivRemOpcode::REMU.opcode().0, 0x257);
    }

    #[test]
    fn test_hint_store_opcodes() {
        assert_eq!(Rv32HintStoreOpcode::HINT_STOREW.opcode().0, 0x260);
        assert_eq!(Rv32HintStoreOpcode::HINT_BUFFER.opcode().0, 0x261);
    }

    #[test]
    fn test_rv32_phantom_values() {
        assert_eq!(Rv32Phantom::HintInput as u16, 0x20);
        assert_eq!(Rv32Phantom::PrintStr as u16, 0x21);
        assert_eq!(Rv32Phantom::HintRandom as u16, 0x22);
        assert_eq!(Rv32Phantom::HintLoadByKey as u16, 0x23);
    }

    #[test]
    fn test_constants() {
        assert_eq!(RV32_REGISTER_NUM_LIMBS, 4);
        assert_eq!(RV32_MEMORY_AS, 2);
    }

    // Test opcode enum traits (Clone, Copy, Debug, PartialEq, Eq)
    #[test]
    fn test_opcode_enum_traits() {
        let op1 = BaseAluOpcode::ADD;
        let op2 = op1; // Copy
        assert_eq!(op1, op2);
        assert_eq!(format!("{:?}", op1), "ADD");
    }
}
