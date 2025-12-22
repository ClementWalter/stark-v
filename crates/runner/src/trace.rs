//! Trace capture for zkVM execution.
//!
//! Each opcode defines its own trace table with specific columns.
//! Registers and memory use a unified Access structure.

use rustc_hash::FxHashMap;

/// Default maximum clock difference allowed between accesses.
/// Must be consistent with max range-check in the prover.
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = 1 << 20; // ~1M cycles

/// Unified access record for both registers and memory.
///
/// - For registers: `addr` is the register index (0-31)
/// - For memory: `addr` is the byte address
#[derive(Debug, Clone, Copy, Default)]
pub struct Access {
    pub addr: u32,
    pub prev: u32,
    pub clk_prev: u32,
    pub next: u32,
    pub clk: u32,
}

// =============================================================================
// Per-opcode trace table structures
// =============================================================================

// R-type ALU traces
#[derive(Debug, Clone)]
pub struct AddTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SubTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SllTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SltTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SltuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct XorTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SrlTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct SraTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct OrTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct AndTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

// I-type ALU traces
#[derive(Debug, Clone)]
pub struct AddiTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct SltiTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct SltiuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct XoriTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct OriTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct AndiTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct SlliTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct SrliTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

#[derive(Debug, Clone)]
pub struct SraiTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

// Load traces
#[derive(Debug, Clone)]
pub struct LbTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct LhTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct LwTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct LbuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct LhuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub mem: Access,
}

// Store traces
#[derive(Debug, Clone)]
pub struct SbTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct ShTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
    pub mem: Access,
}

#[derive(Debug, Clone)]
pub struct SwTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
    pub mem: Access,
}

// Branch traces
#[derive(Debug, Clone)]
pub struct BeqTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct BneTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct BltTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct BgeTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct BltuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct BgeuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rs1: Access,
    pub rs2: Access,
}

// Jump traces
#[derive(Debug, Clone)]
pub struct JalTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
}

#[derive(Debug, Clone)]
pub struct JalrTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
}

// Upper immediate traces
#[derive(Debug, Clone)]
pub struct LuiTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
}

#[derive(Debug, Clone)]
pub struct AuipcTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
}

// M-extension traces
#[derive(Debug, Clone)]
pub struct MulTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct MulhTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct MulhsuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct MulhuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct DivTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct DivuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct RemTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

#[derive(Debug, Clone)]
pub struct RemuTrace {
    pub clk: u32,
    pub pc: u32,
    pub rd: Access,
    pub rs1: Access,
    pub rs2: Access,
}

// =============================================================================
// Tracer: holds all per-opcode trace tables
// =============================================================================

/// Main tracer structure holding all per-opcode trace tables.
#[derive(Debug)]
pub struct Tracer {
    /// Global clock counter, incremented by 1 at each instruction.
    pub clk: u32,
    /// Current program counter (set before each instruction).
    pub pc: u32,
    /// Maximum allowed clock difference between consecutive accesses.
    /// If exceeded, intermediate "catch-up" accesses are generated.
    pub max_clock_diff: u32,

    /// Last access clock for each register (0-31).
    pub reg_clk: [u32; 32],
    /// Last access clock for each memory address.
    pub mem_clk: FxHashMap<u32, u32>,

    // Per-opcode trace tables
    pub add: Vec<AddTrace>,
    pub sub: Vec<SubTrace>,
    pub sll: Vec<SllTrace>,
    pub slt: Vec<SltTrace>,
    pub sltu: Vec<SltuTrace>,
    pub xor: Vec<XorTrace>,
    pub srl: Vec<SrlTrace>,
    pub sra: Vec<SraTrace>,
    pub or: Vec<OrTrace>,
    pub and: Vec<AndTrace>,

    pub addi: Vec<AddiTrace>,
    pub slti: Vec<SltiTrace>,
    pub sltiu: Vec<SltiuTrace>,
    pub xori: Vec<XoriTrace>,
    pub ori: Vec<OriTrace>,
    pub andi: Vec<AndiTrace>,
    pub slli: Vec<SlliTrace>,
    pub srli: Vec<SrliTrace>,
    pub srai: Vec<SraiTrace>,

    pub lb: Vec<LbTrace>,
    pub lh: Vec<LhTrace>,
    pub lw: Vec<LwTrace>,
    pub lbu: Vec<LbuTrace>,
    pub lhu: Vec<LhuTrace>,

    pub sb: Vec<SbTrace>,
    pub sh: Vec<ShTrace>,
    pub sw: Vec<SwTrace>,

    pub beq: Vec<BeqTrace>,
    pub bne: Vec<BneTrace>,
    pub blt: Vec<BltTrace>,
    pub bge: Vec<BgeTrace>,
    pub bltu: Vec<BltuTrace>,
    pub bgeu: Vec<BgeuTrace>,

    pub jal: Vec<JalTrace>,
    pub jalr: Vec<JalrTrace>,

    pub lui: Vec<LuiTrace>,
    pub auipc: Vec<AuipcTrace>,

    pub mul: Vec<MulTrace>,
    pub mulh: Vec<MulhTrace>,
    pub mulhsu: Vec<MulhsuTrace>,
    pub mulhu: Vec<MulhuTrace>,
    pub div: Vec<DivTrace>,
    pub divu: Vec<DivuTrace>,
    pub rem: Vec<RemTrace>,
    pub remu: Vec<RemuTrace>,
}

impl Default for Tracer {
    fn default() -> Self {
        Self {
            clk: 0,
            pc: 0,
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
            reg_clk: [0; 32],
            mem_clk: FxHashMap::default(),

            add: Vec::new(),
            sub: Vec::new(),
            sll: Vec::new(),
            slt: Vec::new(),
            sltu: Vec::new(),
            xor: Vec::new(),
            srl: Vec::new(),
            sra: Vec::new(),
            or: Vec::new(),
            and: Vec::new(),

            addi: Vec::new(),
            slti: Vec::new(),
            sltiu: Vec::new(),
            xori: Vec::new(),
            ori: Vec::new(),
            andi: Vec::new(),
            slli: Vec::new(),
            srli: Vec::new(),
            srai: Vec::new(),

            lb: Vec::new(),
            lh: Vec::new(),
            lw: Vec::new(),
            lbu: Vec::new(),
            lhu: Vec::new(),

            sb: Vec::new(),
            sh: Vec::new(),
            sw: Vec::new(),

            beq: Vec::new(),
            bne: Vec::new(),
            blt: Vec::new(),
            bge: Vec::new(),
            bltu: Vec::new(),
            bgeu: Vec::new(),

            jal: Vec::new(),
            jalr: Vec::new(),

            lui: Vec::new(),
            auipc: Vec::new(),

            mul: Vec::new(),
            mulh: Vec::new(),
            mulhsu: Vec::new(),
            mulhu: Vec::new(),
            div: Vec::new(),
            divu: Vec::new(),
            rem: Vec::new(),
            remu: Vec::new(),
        }
    }
}

impl Tracer {
    /// Create a new tracer with custom max clock diff.
    pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
        Self {
            max_clock_diff,
            ..Default::default()
        }
    }

    /// Create a new tracer with pre-allocated capacity.
    pub fn with_capacity(est_instructions: usize) -> Self {
        // Rough estimate: divide total by number of opcode types
        let cap = est_instructions / 40 + 1;
        Self {
            clk: 0,
            pc: 0,
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
            reg_clk: [0; 32],
            mem_clk: FxHashMap::default(),

            add: Vec::with_capacity(cap),
            sub: Vec::with_capacity(cap),
            sll: Vec::with_capacity(cap),
            slt: Vec::with_capacity(cap),
            sltu: Vec::with_capacity(cap),
            xor: Vec::with_capacity(cap),
            srl: Vec::with_capacity(cap),
            sra: Vec::with_capacity(cap),
            or: Vec::with_capacity(cap),
            and: Vec::with_capacity(cap),

            addi: Vec::with_capacity(cap),
            slti: Vec::with_capacity(cap),
            sltiu: Vec::with_capacity(cap),
            xori: Vec::with_capacity(cap),
            ori: Vec::with_capacity(cap),
            andi: Vec::with_capacity(cap),
            slli: Vec::with_capacity(cap),
            srli: Vec::with_capacity(cap),
            srai: Vec::with_capacity(cap),

            lb: Vec::with_capacity(cap),
            lh: Vec::with_capacity(cap),
            lw: Vec::with_capacity(cap),
            lbu: Vec::with_capacity(cap),
            lhu: Vec::with_capacity(cap),

            sb: Vec::with_capacity(cap),
            sh: Vec::with_capacity(cap),
            sw: Vec::with_capacity(cap),

            beq: Vec::with_capacity(cap),
            bne: Vec::with_capacity(cap),
            blt: Vec::with_capacity(cap),
            bge: Vec::with_capacity(cap),
            bltu: Vec::with_capacity(cap),
            bgeu: Vec::with_capacity(cap),

            jal: Vec::with_capacity(cap),
            jalr: Vec::with_capacity(cap),

            lui: Vec::with_capacity(cap),
            auipc: Vec::with_capacity(cap),

            mul: Vec::with_capacity(cap),
            mulh: Vec::with_capacity(cap),
            mulhsu: Vec::with_capacity(cap),
            mulhu: Vec::with_capacity(cap),
            div: Vec::with_capacity(cap),
            divu: Vec::with_capacity(cap),
            rem: Vec::with_capacity(cap),
            remu: Vec::with_capacity(cap),
        }
    }

    // =========================================================================
    // Gap-filling trace methods
    // =========================================================================

    /// Generate intermediate accesses to bridge a clock gap.
    /// Returns accesses from `clk_prev` to just before `target_clk`.
    fn generate_intermediates(
        &self,
        addr: u32,
        value: u32,
        clk_prev: u32,
        target_clk: u32,
    ) -> (Vec<Access>, u32) {
        let mut accesses = Vec::new();
        let mut current_clk = clk_prev;

        while target_clk.saturating_sub(current_clk) > self.max_clock_diff {
            let next_clk = current_clk.saturating_add(self.max_clock_diff);
            accesses.push(Access {
                addr,
                prev: value,
                clk_prev: current_clk,
                next: value,
                clk: next_clk,
            });
            current_clk = next_clk;
        }

        (accesses, current_clk)
    }

    /// Trace a register access with gap-filling.
    /// Returns all accesses including intermediates and the final access.
    pub fn trace_reg_access(&mut self, idx: u8, prev: u32, next: u32) -> Vec<Access> {
        let clk_prev = self.reg_clk[idx as usize];
        let addr = idx as u32;

        // Generate intermediate catch-up accesses
        let (mut accesses, final_clk_prev) =
            self.generate_intermediates(addr, prev, clk_prev, self.clk);

        // Update reg_clk for intermediates
        if !accesses.is_empty() {
            self.reg_clk[idx as usize] = final_clk_prev;
        }

        // Add the final access
        accesses.push(Access {
            addr,
            prev,
            clk_prev: final_clk_prev,
            next,
            clk: self.clk,
        });

        // Update the register's clock
        self.reg_clk[idx as usize] = self.clk;

        accesses
    }

    /// Trace a memory byte access with gap-filling.
    /// Returns all accesses including intermediates and the final access.
    pub fn trace_mem_access(&mut self, addr: u32, prev: u32, next: u32) -> Vec<Access> {
        let clk_prev = self.mem_clk.get(&addr).copied().unwrap_or(0);

        // Generate intermediate catch-up accesses
        let (mut accesses, final_clk_prev) =
            self.generate_intermediates(addr, prev, clk_prev, self.clk);

        // Update mem_clk for intermediates
        if !accesses.is_empty() {
            self.mem_clk.insert(addr, final_clk_prev);
        }

        // Add the final access
        accesses.push(Access {
            addr,
            prev,
            clk_prev: final_clk_prev,
            next,
            clk: self.clk,
        });

        // Update the memory byte's clock
        self.mem_clk.insert(addr, self.clk);

        accesses
    }

    /// Total number of traced instructions.
    pub fn total_traces(&self) -> usize {
        self.add.len()
            + self.sub.len()
            + self.sll.len()
            + self.slt.len()
            + self.sltu.len()
            + self.xor.len()
            + self.srl.len()
            + self.sra.len()
            + self.or.len()
            + self.and.len()
            + self.addi.len()
            + self.slti.len()
            + self.sltiu.len()
            + self.xori.len()
            + self.ori.len()
            + self.andi.len()
            + self.slli.len()
            + self.srli.len()
            + self.srai.len()
            + self.lb.len()
            + self.lh.len()
            + self.lw.len()
            + self.lbu.len()
            + self.lhu.len()
            + self.sb.len()
            + self.sh.len()
            + self.sw.len()
            + self.beq.len()
            + self.bne.len()
            + self.blt.len()
            + self.bge.len()
            + self.bltu.len()
            + self.bgeu.len()
            + self.jal.len()
            + self.jalr.len()
            + self.lui.len()
            + self.auipc.len()
            + self.mul.len()
            + self.mulh.len()
            + self.mulhsu.len()
            + self.mulhu.len()
            + self.div.len()
            + self.divu.len()
            + self.rem.len()
            + self.remu.len()
    }
}

// =============================================================================
// Declarative trace! macro
// =============================================================================

/// Trace macro for recording opcode execution.
///
/// Usage: `trace!(opcode: field1, field2, ...)`
///
/// The macro pushes a new trace row to the appropriate table.
#[macro_export]
macro_rules! trace {
    (add: $($field:ident),+ $(,)?) => {
        tracer.add.push($crate::trace::AddTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sub: $($field:ident),+ $(,)?) => {
        tracer.sub.push($crate::trace::SubTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sll: $($field:ident),+ $(,)?) => {
        tracer.sll.push($crate::trace::SllTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (slt: $($field:ident),+ $(,)?) => {
        tracer.slt.push($crate::trace::SltTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sltu: $($field:ident),+ $(,)?) => {
        tracer.sltu.push($crate::trace::SltuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (xor: $($field:ident),+ $(,)?) => {
        tracer.xor.push($crate::trace::XorTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (srl: $($field:ident),+ $(,)?) => {
        tracer.srl.push($crate::trace::SrlTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sra: $($field:ident),+ $(,)?) => {
        tracer.sra.push($crate::trace::SraTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (or: $($field:ident),+ $(,)?) => {
        tracer.or.push($crate::trace::OrTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (and: $($field:ident),+ $(,)?) => {
        tracer.and.push($crate::trace::AndTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (addi: $($field:ident),+ $(,)?) => {
        tracer.addi.push($crate::trace::AddiTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (slti: $($field:ident),+ $(,)?) => {
        tracer.slti.push($crate::trace::SltiTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sltiu: $($field:ident),+ $(,)?) => {
        tracer.sltiu.push($crate::trace::SltiuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (xori: $($field:ident),+ $(,)?) => {
        tracer.xori.push($crate::trace::XoriTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (ori: $($field:ident),+ $(,)?) => {
        tracer.ori.push($crate::trace::OriTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (andi: $($field:ident),+ $(,)?) => {
        tracer.andi.push($crate::trace::AndiTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (slli: $($field:ident),+ $(,)?) => {
        tracer.slli.push($crate::trace::SlliTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (srli: $($field:ident),+ $(,)?) => {
        tracer.srli.push($crate::trace::SrliTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (srai: $($field:ident),+ $(,)?) => {
        tracer.srai.push($crate::trace::SraiTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lb: $($field:ident),+ $(,)?) => {
        tracer.lb.push($crate::trace::LbTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lh: $($field:ident),+ $(,)?) => {
        tracer.lh.push($crate::trace::LhTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lw: $($field:ident),+ $(,)?) => {
        tracer.lw.push($crate::trace::LwTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lbu: $($field:ident),+ $(,)?) => {
        tracer.lbu.push($crate::trace::LbuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lhu: $($field:ident),+ $(,)?) => {
        tracer.lhu.push($crate::trace::LhuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sb: $($field:ident),+ $(,)?) => {
        tracer.sb.push($crate::trace::SbTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sh: $($field:ident),+ $(,)?) => {
        tracer.sh.push($crate::trace::ShTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (sw: $($field:ident),+ $(,)?) => {
        tracer.sw.push($crate::trace::SwTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (beq: $($field:ident),+ $(,)?) => {
        tracer.beq.push($crate::trace::BeqTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (bne: $($field:ident),+ $(,)?) => {
        tracer.bne.push($crate::trace::BneTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (blt: $($field:ident),+ $(,)?) => {
        tracer.blt.push($crate::trace::BltTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (bge: $($field:ident),+ $(,)?) => {
        tracer.bge.push($crate::trace::BgeTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (bltu: $($field:ident),+ $(,)?) => {
        tracer.bltu.push($crate::trace::BltuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (bgeu: $($field:ident),+ $(,)?) => {
        tracer.bgeu.push($crate::trace::BgeuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (jal: $($field:ident),+ $(,)?) => {
        tracer.jal.push($crate::trace::JalTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (jalr: $($field:ident),+ $(,)?) => {
        tracer.jalr.push($crate::trace::JalrTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (lui: $($field:ident),+ $(,)?) => {
        tracer.lui.push($crate::trace::LuiTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (auipc: $($field:ident),+ $(,)?) => {
        tracer.auipc.push($crate::trace::AuipcTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (mul: $($field:ident),+ $(,)?) => {
        tracer.mul.push($crate::trace::MulTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (mulh: $($field:ident),+ $(,)?) => {
        tracer.mulh.push($crate::trace::MulhTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (mulhsu: $($field:ident),+ $(,)?) => {
        tracer.mulhsu.push($crate::trace::MulhsuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (mulhu: $($field:ident),+ $(,)?) => {
        tracer.mulhu.push($crate::trace::MulhuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (div: $($field:ident),+ $(,)?) => {
        tracer.div.push($crate::trace::DivTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (divu: $($field:ident),+ $(,)?) => {
        tracer.divu.push($crate::trace::DivuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (rem: $($field:ident),+ $(,)?) => {
        tracer.rem.push($crate::trace::RemTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
    (remu: $($field:ident),+ $(,)?) => {
        tracer.remu.push($crate::trace::RemuTrace { clk: tracer.clk, pc: tracer.pc, $($field),+ });
    };
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    // =========================================================================
    // Tracer Construction
    // =========================================================================

    #[test]
    fn test_default_tracer() {
        let tracer = Tracer::default();
        assert_eq!(tracer.clk, 0);
        assert_eq!(tracer.pc, 0);
        assert_eq!(tracer.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
        assert_eq!(tracer.reg_clk, [0; 32]);
        assert!(tracer.mem_clk.is_empty());
    }

    #[test]
    fn test_with_max_clock_diff() {
        let tracer = Tracer::with_max_clock_diff(100);
        assert_eq!(tracer.max_clock_diff, 100);
        assert_eq!(tracer.clk, 0);
    }

    // =========================================================================
    // Memory Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_mem_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        let accesses = tracer.trace_mem_access(100, 0x42, 0x42);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].addr, 100);
        assert_eq!(accesses[0].prev, 0x42);
        assert_eq!(accesses[0].next, 0x42);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, 10);
    }

    #[test]
    fn test_trace_mem_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clk = 1;
        tracer.trace_mem_access(100, 0x11, 0x11);

        tracer.clk = 2;
        let accesses = tracer.trace_mem_access(100, 0x11, 0x22);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].clk_prev, 1);
        assert_eq!(accesses[0].clk, 2);
        assert_eq!(accesses[0].prev, 0x11);
        assert_eq!(accesses[0].next, 0x22);
    }

    #[test]
    fn test_trace_mem_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0x42, 0x42);

        tracer.clk = 350;
        let accesses = tracer.trace_mem_access(100, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates + 1 final
        assert_eq!(
            accesses.len(),
            4,
            "Expected 4 accesses, got {}",
            accesses.len()
        );

        // Verify all clock diffs are within max_clock_diff
        for access in &accesses {
            let diff = access.clk.saturating_sub(access.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} exceeds max_clock_diff 100",
                diff
            );
        }
    }

    #[test]
    fn test_trace_mem_access_exact_max_diff() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = 100;
        let accesses = tracer.trace_mem_access(100, 0, 0);

        // Exactly at max_clock_diff - no intermediate needed
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, 100);
    }

    #[test]
    fn test_trace_mem_access_preserves_value() {
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0xAB, 0xAB);

        tracer.clk = 200;
        let accesses = tracer.trace_mem_access(100, 0xAB, 0xAB);

        // All intermediate accesses should preserve the value
        for access in &accesses {
            assert_eq!(access.prev, 0xAB);
            assert_eq!(access.next, 0xAB);
        }
    }

    #[test]
    fn test_trace_mem_access_updates_mem_clk() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        tracer.trace_mem_access(100, 0, 0);

        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
    }

    // =========================================================================
    // Register Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_reg_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        let accesses = tracer.trace_reg_access(5, 0x42, 0x42);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].addr, 5);
        assert_eq!(accesses[0].prev, 0x42);
        assert_eq!(accesses[0].next, 0x42);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, 10);
    }

    #[test]
    fn test_trace_reg_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clk = 1;
        tracer.trace_reg_access(5, 0x11, 0x11);

        tracer.clk = 2;
        let accesses = tracer.trace_reg_access(5, 0x11, 0x22);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].clk_prev, 1);
        assert_eq!(accesses[0].clk, 2);
        assert_eq!(accesses[0].prev, 0x11);
        assert_eq!(accesses[0].next, 0x22);
    }

    #[test]
    fn test_trace_reg_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_reg_access(5, 0x42, 0x42);

        tracer.clk = 350;
        let accesses = tracer.trace_reg_access(5, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates + 1 final
        assert_eq!(
            accesses.len(),
            4,
            "Expected 4 accesses, got {}",
            accesses.len()
        );

        // Verify all clock diffs are within max_clock_diff
        for access in &accesses {
            let diff = access.clk.saturating_sub(access.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} exceeds max_clock_diff 100",
                diff
            );
        }
    }

    #[test]
    fn test_trace_reg_access_x0() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        // x0 can still be traced - the caller handles x0 semantics
        let accesses = tracer.trace_reg_access(0, 0, 0);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].addr, 0);
        assert_eq!(accesses[0].prev, 0);
        assert_eq!(accesses[0].next, 0);
    }

    #[test]
    fn test_trace_reg_access_updates_reg_clk() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        tracer.trace_reg_access(5, 0, 0);

        assert_eq!(tracer.reg_clk[5], 10);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_max_clock_diff_one() {
        let mut tracer = Tracer::with_max_clock_diff(1);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = 5;
        let accesses = tracer.trace_mem_access(100, 0, 0);

        // With max_clock_diff=1, gap of 5 needs 5 accesses
        assert_eq!(accesses.len(), 5);

        // Verify each step is exactly 1
        for access in &accesses {
            let diff = access.clk - access.clk_prev;
            assert_eq!(diff, 1);
        }
    }

    #[test]
    fn test_max_clock_diff_max() {
        let mut tracer = Tracer::with_max_clock_diff(u32::MAX);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = u32::MAX - 1;
        let accesses = tracer.trace_mem_access(100, 0, 0);

        // No intermediate ever needed
        assert_eq!(accesses.len(), 1);
    }
}
