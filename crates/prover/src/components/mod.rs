//! Component system for tracer-backed and preprocessed AIR components.

pub mod mem_clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod preprocessed;
pub mod program;
pub mod reg_clock_update;

pub use opcodes::{
    auipc, base_alu_imm, base_alu_reg, branch_eq, branch_lt, div, jal, jalr, load_store, lt_imm,
    lt_reg, lui, mul, mulh, shifts_imm, shifts_reg,
};

use serde::{Deserialize, Serialize};
use stwo::core::ColumnVec;
use stwo::core::air::Component;
use stwo::core::channel::Channel;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::core::pcs::TreeVec;
use stwo::prover::backend::Column;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_constraint_framework::relation_tracker::RelationSummary;

use crate::relations::Relations;

type TraceColumns = ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>;

/// Aggregate of all main trace columns.
pub struct Traces {
    /// AUIPC trace rows.
    pub auipc: TraceColumns,
    /// Base ALU immediate trace rows.
    pub base_alu_imm: TraceColumns,
    /// Base ALU register trace rows.
    pub base_alu_reg: TraceColumns,
    /// Branch equality trace rows.
    pub branch_eq: TraceColumns,
    /// Branch comparison trace rows.
    pub branch_lt: TraceColumns,
    /// Division trace rows.
    pub div: TraceColumns,
    /// JAL trace rows.
    pub jal: TraceColumns,
    /// JALR trace rows.
    pub jalr: TraceColumns,
    /// Load/store trace rows.
    pub load_store: TraceColumns,
    /// Immediate less-than trace rows.
    pub lt_imm: TraceColumns,
    /// Register less-than trace rows.
    pub lt_reg: TraceColumns,
    /// LUI trace rows.
    pub lui: TraceColumns,
    /// Multiplication trace rows.
    pub mul: TraceColumns,
    /// High multiplication trace rows.
    pub mulh: TraceColumns,
    /// Immediate shift trace rows.
    pub shifts_imm: TraceColumns,
    /// Register shift trace rows.
    pub shifts_reg: TraceColumns,
    /// Program trace rows.
    pub program: TraceColumns,
    /// Memory commitment trace rows.
    pub memory: TraceColumns,
    /// Merkle node trace rows.
    pub merkle: TraceColumns,
    /// Poseidon2 input trace rows.
    pub poseidon2: TraceColumns,
    /// Memory clock update trace rows.
    pub mem_clock_update: TraceColumns,
    /// Register clock update trace rows.
    pub reg_clock_update: TraceColumns,
    /// Preprocessed multiplicity traces.
    pub preprocessed: preprocessed::Traces,
}

impl Traces {
    /// Returns the maximum log_size across all traces.
    pub fn max_log_size(&self) -> u32 {
        self.log_sizes().into_iter().max().unwrap_or(4)
    }

    /// Returns all log_sizes from main trace columns.
    pub fn log_sizes(&self) -> Vec<u32> {
        let mut sizes = vec![];
        push_trace_log_size(&mut sizes, &self.auipc);
        push_trace_log_size(&mut sizes, &self.base_alu_imm);
        push_trace_log_size(&mut sizes, &self.base_alu_reg);
        push_trace_log_size(&mut sizes, &self.branch_eq);
        push_trace_log_size(&mut sizes, &self.branch_lt);
        push_trace_log_size(&mut sizes, &self.div);
        push_trace_log_size(&mut sizes, &self.jal);
        push_trace_log_size(&mut sizes, &self.jalr);
        push_trace_log_size(&mut sizes, &self.load_store);
        push_trace_log_size(&mut sizes, &self.lt_imm);
        push_trace_log_size(&mut sizes, &self.lt_reg);
        push_trace_log_size(&mut sizes, &self.lui);
        push_trace_log_size(&mut sizes, &self.mul);
        push_trace_log_size(&mut sizes, &self.mulh);
        push_trace_log_size(&mut sizes, &self.shifts_imm);
        push_trace_log_size(&mut sizes, &self.shifts_reg);
        push_trace_log_size(&mut sizes, &self.program);
        push_trace_log_size(&mut sizes, &self.memory);
        push_trace_log_size(&mut sizes, &self.merkle);
        push_trace_log_size(&mut sizes, &self.poseidon2);
        push_trace_log_size(&mut sizes, &self.mem_clock_update);
        push_trace_log_size(&mut sizes, &self.reg_clock_update);
        sizes.extend(self.preprocessed.log_sizes());
        sizes
    }

    /// Clone all columns into commitment order.
    pub fn columns_cloned(&self) -> TraceColumns {
        let mut columns = vec![];
        columns.extend(self.auipc.clone());
        columns.extend(self.base_alu_imm.clone());
        columns.extend(self.base_alu_reg.clone());
        columns.extend(self.branch_eq.clone());
        columns.extend(self.branch_lt.clone());
        columns.extend(self.div.clone());
        columns.extend(self.jal.clone());
        columns.extend(self.jalr.clone());
        columns.extend(self.load_store.clone());
        columns.extend(self.lt_imm.clone());
        columns.extend(self.lt_reg.clone());
        columns.extend(self.lui.clone());
        columns.extend(self.mul.clone());
        columns.extend(self.mulh.clone());
        columns.extend(self.shifts_imm.clone());
        columns.extend(self.shifts_reg.clone());
        columns.extend(self.program.clone());
        columns.extend(self.memory.clone());
        columns.extend(self.merkle.clone());
        columns.extend(self.poseidon2.clone());
        columns.extend(self.mem_clock_update.clone());
        columns.extend(self.reg_clock_update.clone());
        columns.extend(self.preprocessed.columns_cloned());
        columns
    }
}

fn push_trace_log_size(sizes: &mut Vec<u32>, trace: &TraceColumns) {
    if let Some(first) = trace.first() {
        sizes.push(first.domain.log_size());
    }
}

fn trace_log_size(trace: &TraceColumns) -> u32 {
    trace
        .first()
        .map(|eval| eval.domain.log_size())
        .unwrap_or(0)
}

/// Claim containing log_size for each component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// AUIPC trace log size.
    pub auipc: u32,
    /// Base ALU immediate trace log size.
    pub base_alu_imm: u32,
    /// Base ALU register trace log size.
    pub base_alu_reg: u32,
    /// Branch equality trace log size.
    pub branch_eq: u32,
    /// Branch comparison trace log size.
    pub branch_lt: u32,
    /// Division trace log size.
    pub div: u32,
    /// JAL trace log size.
    pub jal: u32,
    /// JALR trace log size.
    pub jalr: u32,
    /// Load/store trace log size.
    pub load_store: u32,
    /// Immediate less-than trace log size.
    pub lt_imm: u32,
    /// Register less-than trace log size.
    pub lt_reg: u32,
    /// LUI trace log size.
    pub lui: u32,
    /// Multiplication trace log size.
    pub mul: u32,
    /// High multiplication trace log size.
    pub mulh: u32,
    /// Immediate shift trace log size.
    pub shifts_imm: u32,
    /// Register shift trace log size.
    pub shifts_reg: u32,
    /// Program trace log size.
    pub program: u32,
    /// Memory commitment log size.
    pub memory: u32,
    /// Merkle trace log size.
    pub merkle: u32,
    /// Poseidon2 trace log size.
    pub poseidon2: u32,
    /// Memory clock update trace log size.
    pub mem_clock_update: u32,
    /// Register clock update trace log size.
    pub reg_clock_update: u32,
    /// Preprocessed claims.
    pub preprocessed: preprocessed::Claim,
}

impl From<&Traces> for Claim {
    fn from(traces: &Traces) -> Self {
        Self {
            auipc: trace_log_size(&traces.auipc),
            base_alu_imm: trace_log_size(&traces.base_alu_imm),
            base_alu_reg: trace_log_size(&traces.base_alu_reg),
            branch_eq: trace_log_size(&traces.branch_eq),
            branch_lt: trace_log_size(&traces.branch_lt),
            div: trace_log_size(&traces.div),
            jal: trace_log_size(&traces.jal),
            jalr: trace_log_size(&traces.jalr),
            load_store: trace_log_size(&traces.load_store),
            lt_imm: trace_log_size(&traces.lt_imm),
            lt_reg: trace_log_size(&traces.lt_reg),
            lui: trace_log_size(&traces.lui),
            mul: trace_log_size(&traces.mul),
            mulh: trace_log_size(&traces.mulh),
            shifts_imm: trace_log_size(&traces.shifts_imm),
            shifts_reg: trace_log_size(&traces.shifts_reg),
            program: trace_log_size(&traces.program),
            memory: trace_log_size(&traces.memory),
            merkle: trace_log_size(&traces.merkle),
            poseidon2: trace_log_size(&traces.poseidon2),
            mem_clock_update: trace_log_size(&traces.mem_clock_update),
            reg_clock_update: trace_log_size(&traces.reg_clock_update),
            preprocessed: (&traces.preprocessed).into(),
        }
    }
}

impl Claim {
    /// Mix claim into the channel in commitment order.
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.auipc as u64);
        channel.mix_u64(self.base_alu_imm as u64);
        channel.mix_u64(self.base_alu_reg as u64);
        channel.mix_u64(self.branch_eq as u64);
        channel.mix_u64(self.branch_lt as u64);
        channel.mix_u64(self.div as u64);
        channel.mix_u64(self.jal as u64);
        channel.mix_u64(self.jalr as u64);
        channel.mix_u64(self.load_store as u64);
        channel.mix_u64(self.lt_imm as u64);
        channel.mix_u64(self.lt_reg as u64);
        channel.mix_u64(self.lui as u64);
        channel.mix_u64(self.mul as u64);
        channel.mix_u64(self.mulh as u64);
        channel.mix_u64(self.shifts_imm as u64);
        channel.mix_u64(self.shifts_reg as u64);
        channel.mix_u64(self.program as u64);
        channel.mix_u64(self.memory as u64);
        channel.mix_u64(self.merkle as u64);
        channel.mix_u64(self.poseidon2 as u64);
        channel.mix_u64(self.mem_clock_update as u64);
        channel.mix_u64(self.reg_clock_update as u64);
        self.preprocessed.mix_into(channel);
    }

    /// Log sizes for all main trace columns in commitment order.
    pub fn main_trace_log_sizes(&self) -> Vec<u32> {
        let mut sizes = vec![];
        extend_repeated_log_size::<runner::trace::prover_columns::AuipcColumns<()>>(
            &mut sizes, self.auipc,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::BaseAluImmColumns<()>>(
            &mut sizes,
            self.base_alu_imm,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::BaseAluRegColumns<()>>(
            &mut sizes,
            self.base_alu_reg,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::BranchEqColumns<()>>(
            &mut sizes,
            self.branch_eq,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::BranchLtColumns<()>>(
            &mut sizes,
            self.branch_lt,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::DivColumns<()>>(
            &mut sizes, self.div,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::JalColumns<()>>(
            &mut sizes, self.jal,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::JalrColumns<()>>(
            &mut sizes, self.jalr,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::LoadStoreColumns<()>>(
            &mut sizes,
            self.load_store,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::LtImmColumns<()>>(
            &mut sizes,
            self.lt_imm,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::LtRegColumns<()>>(
            &mut sizes,
            self.lt_reg,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::LuiColumns<()>>(
            &mut sizes, self.lui,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::MulColumns<()>>(
            &mut sizes, self.mul,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::MulhColumns<()>>(
            &mut sizes, self.mulh,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::ShiftsImmColumns<()>>(
            &mut sizes,
            self.shifts_imm,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::ShiftsRegColumns<()>>(
            &mut sizes,
            self.shifts_reg,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::ProgramColumns<()>>(
            &mut sizes,
            self.program,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::MemoryColumns<()>>(
            &mut sizes,
            self.memory,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::MerkleColumns<()>>(
            &mut sizes,
            self.merkle,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::Poseidon2Columns<()>>(
            &mut sizes,
            self.poseidon2,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::MemClockUpdateColumns<()>>(
            &mut sizes,
            self.mem_clock_update,
        );
        extend_repeated_log_size::<runner::trace::prover_columns::RegClockUpdateColumns<()>>(
            &mut sizes,
            self.reg_clock_update,
        );
        sizes.extend(self.preprocessed.log_sizes());
        sizes
    }
}

trait ColumnCount {
    const SIZE: usize;
}

macro_rules! impl_column_count {
    ($($columns:ty),* $(,)?) => {
        $(
            impl ColumnCount for $columns {
                const SIZE: usize = <$columns>::SIZE;
            }
        )*
    };
}

impl_column_count!(
    runner::trace::prover_columns::AuipcColumns<()>,
    runner::trace::prover_columns::BaseAluImmColumns<()>,
    runner::trace::prover_columns::BaseAluRegColumns<()>,
    runner::trace::prover_columns::BranchEqColumns<()>,
    runner::trace::prover_columns::BranchLtColumns<()>,
    runner::trace::prover_columns::DivColumns<()>,
    runner::trace::prover_columns::JalColumns<()>,
    runner::trace::prover_columns::JalrColumns<()>,
    runner::trace::prover_columns::LoadStoreColumns<()>,
    runner::trace::prover_columns::LtImmColumns<()>,
    runner::trace::prover_columns::LtRegColumns<()>,
    runner::trace::prover_columns::LuiColumns<()>,
    runner::trace::prover_columns::MulColumns<()>,
    runner::trace::prover_columns::MulhColumns<()>,
    runner::trace::prover_columns::ShiftsImmColumns<()>,
    runner::trace::prover_columns::ShiftsRegColumns<()>,
    runner::trace::prover_columns::ProgramColumns<()>,
    runner::trace::prover_columns::MemoryColumns<()>,
    runner::trace::prover_columns::MerkleColumns<()>,
    runner::trace::prover_columns::Poseidon2Columns<()>,
    runner::trace::prover_columns::MemClockUpdateColumns<()>,
    runner::trace::prover_columns::RegClockUpdateColumns<()>,
);

fn extend_repeated_log_size<C: ColumnCount>(sizes: &mut Vec<u32>, log_size: u32) {
    sizes.extend(std::iter::repeat_n(log_size, C::SIZE));
}

/// Aggregate of all claimed sums from interaction traces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClaimedSum {
    /// AUIPC claimed sum.
    pub auipc: QM31,
    /// Base ALU immediate claimed sum.
    pub base_alu_imm: QM31,
    /// Base ALU register claimed sum.
    pub base_alu_reg: QM31,
    /// Branch equality claimed sum.
    pub branch_eq: QM31,
    /// Branch comparison claimed sum.
    pub branch_lt: QM31,
    /// Division claimed sum.
    pub div: QM31,
    /// JAL claimed sum.
    pub jal: QM31,
    /// JALR claimed sum.
    pub jalr: QM31,
    /// Load/store claimed sum.
    pub load_store: QM31,
    /// Immediate less-than claimed sum.
    pub lt_imm: QM31,
    /// Register less-than claimed sum.
    pub lt_reg: QM31,
    /// LUI claimed sum.
    pub lui: QM31,
    /// Multiplication claimed sum.
    pub mul: QM31,
    /// High multiplication claimed sum.
    pub mulh: QM31,
    /// Immediate shift claimed sum.
    pub shifts_imm: QM31,
    /// Register shift claimed sum.
    pub shifts_reg: QM31,
    /// Claimed sum for program component.
    pub program: QM31,
    /// Claimed sum for memory component.
    pub memory: QM31,
    /// Claimed sum for merkle component.
    pub merkle: QM31,
    /// Claimed sum for poseidon2 component.
    pub poseidon2: QM31,
    /// Claimed sum for memory clock update component.
    pub mem_clock_update: QM31,
    /// Claimed sum for register clock update component.
    pub reg_clock_update: QM31,
    /// Claimed sums from preprocessed components.
    pub preprocessed: preprocessed::ClaimedSum,
}

impl ClaimedSum {
    /// Sum all claimed values.
    pub fn total(&self) -> QM31 {
        self.auipc
            + self.base_alu_imm
            + self.base_alu_reg
            + self.branch_eq
            + self.branch_lt
            + self.div
            + self.jal
            + self.jalr
            + self.load_store
            + self.lt_imm
            + self.lt_reg
            + self.lui
            + self.mul
            + self.mulh
            + self.shifts_imm
            + self.shifts_reg
            + self.program
            + self.memory
            + self.merkle
            + self.poseidon2
            + self.mem_clock_update
            + self.reg_clock_update
            + self.preprocessed.sum()
    }

    /// Mix claimed sums into the channel in commitment order.
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[
            self.auipc,
            self.base_alu_imm,
            self.base_alu_reg,
            self.branch_eq,
            self.branch_lt,
            self.div,
            self.jal,
            self.jalr,
            self.load_store,
            self.lt_imm,
            self.lt_reg,
            self.lui,
            self.mul,
            self.mulh,
            self.shifts_imm,
            self.shifts_reg,
            self.program,
            self.memory,
            self.merkle,
            self.poseidon2,
            self.mem_clock_update,
            self.reg_clock_update,
        ]);
        self.preprocessed.mix_into(channel);
    }
}

macro_rules! new_component {
    ($module:ident, $claim:ident, $location_allocator:ident, $relations:ident, $claimed_sum:ident) => {
        $module::air::Component::new(
            $location_allocator,
            $module::air::Eval {
                log_size: $claim.$module,
                relations: $relations.clone(),
            },
            $claimed_sum.$module,
        )
    };
}

macro_rules! assert_trace_constraints {
    ($traces:ident, $relations:ident, $module:ident, $label:literal) => {
        if !$traces.$module.is_empty() {
            let log_size = $traces
                .$module
                .first()
                .map(|t| t.domain.log_size())
                .unwrap_or(0);
            if log_size > 0 {
                let (interaction_trace, claimed_sum) =
                    $module::witness::gen_interaction_trace(&$traces.$module, $relations);
                let trace_tree =
                    TreeVec::new(vec![vec![], $traces.$module.clone(), interaction_trace]);
                let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                let eval = $module::air::Eval {
                    log_size,
                    relations: $relations.clone(),
                };
                info!(
                    concat!("Testing ", $label, " constraints (log_size={})"),
                    log_size
                );
                assert_constraints_on_polys(
                    &trace_polys,
                    CanonicCoset::new(log_size),
                    |assert_eval| {
                        eval.evaluate(assert_eval);
                    },
                    claimed_sum,
                );
                info!(concat!($label, " constraints OK"));
            }
        }
    };
}

/// Aggregate of all AIR components.
pub struct Components {
    /// AUIPC component.
    pub auipc: auipc::air::Component,
    /// Base ALU immediate component.
    pub base_alu_imm: base_alu_imm::air::Component,
    /// Base ALU register component.
    pub base_alu_reg: base_alu_reg::air::Component,
    /// Branch equality component.
    pub branch_eq: branch_eq::air::Component,
    /// Branch comparison component.
    pub branch_lt: branch_lt::air::Component,
    /// Division component.
    pub div: div::air::Component,
    /// JAL component.
    pub jal: jal::air::Component,
    /// JALR component.
    pub jalr: jalr::air::Component,
    /// Load/store component.
    pub load_store: load_store::air::Component,
    /// Immediate less-than component.
    pub lt_imm: lt_imm::air::Component,
    /// Register less-than component.
    pub lt_reg: lt_reg::air::Component,
    /// LUI component.
    pub lui: lui::air::Component,
    /// Multiplication component.
    pub mul: mul::air::Component,
    /// High multiplication component.
    pub mulh: mulh::air::Component,
    /// Immediate shift component.
    pub shifts_imm: shifts_imm::air::Component,
    /// Register shift component.
    pub shifts_reg: shifts_reg::air::Component,
    /// Program component.
    pub program: program::air::Component,
    /// Memory commitment component.
    pub memory: memory::air::Component,
    /// Merkle component.
    pub merkle: merkle::air::Component,
    /// Poseidon2 component.
    pub poseidon2: poseidon2::air::Component,
    /// Memory clock update component.
    pub mem_clock_update: mem_clock_update::air::Component,
    /// Register clock update component.
    pub reg_clock_update: reg_clock_update::air::Component,
    /// Preprocessed components.
    pub preprocessed: preprocessed::Components,
}

impl Components {
    /// Create all AIR components.
    pub fn new(
        claim: &Claim,
        location_allocator: &mut TraceLocationAllocator,
        relations: Relations,
        claimed_sum: &ClaimedSum,
    ) -> Self {
        let auipc = new_component!(auipc, claim, location_allocator, relations, claimed_sum);
        let base_alu_imm = new_component!(
            base_alu_imm,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let base_alu_reg = new_component!(
            base_alu_reg,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let branch_eq =
            new_component!(branch_eq, claim, location_allocator, relations, claimed_sum);
        let branch_lt =
            new_component!(branch_lt, claim, location_allocator, relations, claimed_sum);
        let div = new_component!(div, claim, location_allocator, relations, claimed_sum);
        let jal = new_component!(jal, claim, location_allocator, relations, claimed_sum);
        let jalr = new_component!(jalr, claim, location_allocator, relations, claimed_sum);
        let load_store = new_component!(
            load_store,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let lt_imm = new_component!(lt_imm, claim, location_allocator, relations, claimed_sum);
        let lt_reg = new_component!(lt_reg, claim, location_allocator, relations, claimed_sum);
        let lui = new_component!(lui, claim, location_allocator, relations, claimed_sum);
        let mul = new_component!(mul, claim, location_allocator, relations, claimed_sum);
        let mulh = new_component!(mulh, claim, location_allocator, relations, claimed_sum);
        let shifts_imm = new_component!(
            shifts_imm,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let shifts_reg = new_component!(
            shifts_reg,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let program = new_component!(program, claim, location_allocator, relations, claimed_sum);
        let memory = new_component!(memory, claim, location_allocator, relations, claimed_sum);
        let merkle = new_component!(merkle, claim, location_allocator, relations, claimed_sum);
        let poseidon2 =
            new_component!(poseidon2, claim, location_allocator, relations, claimed_sum);
        let mem_clock_update = new_component!(
            mem_clock_update,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let reg_clock_update = new_component!(
            reg_clock_update,
            claim,
            location_allocator,
            relations,
            claimed_sum
        );
        let preprocessed = preprocessed::Components::new(
            &claim.preprocessed,
            location_allocator,
            relations,
            &claimed_sum.preprocessed,
        );

        Self {
            auipc,
            base_alu_imm,
            base_alu_reg,
            branch_eq,
            branch_lt,
            div,
            jal,
            jalr,
            load_store,
            lt_imm,
            lt_reg,
            lui,
            mul,
            mulh,
            shifts_imm,
            shifts_reg,
            program,
            memory,
            merkle,
            poseidon2,
            mem_clock_update,
            reg_clock_update,
            preprocessed,
        }
    }

    /// Get all components as trait objects for proving.
    pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
        let mut provers: Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> = vec![
            &self.auipc,
            &self.base_alu_imm,
            &self.base_alu_reg,
            &self.branch_eq,
            &self.branch_lt,
            &self.div,
            &self.jal,
            &self.jalr,
            &self.load_store,
            &self.lt_imm,
            &self.lt_reg,
            &self.lui,
            &self.mul,
            &self.mulh,
            &self.shifts_imm,
            &self.shifts_reg,
            &self.program,
            &self.memory,
            &self.merkle,
            &self.poseidon2,
            &self.mem_clock_update,
            &self.reg_clock_update,
        ];
        provers.extend(self.preprocessed.provers());
        provers
    }

    /// Get all components as trait objects for verification.
    pub fn verifiers(&self) -> Vec<&dyn Component> {
        let mut verifiers: Vec<&dyn Component> = vec![
            &self.auipc,
            &self.base_alu_imm,
            &self.base_alu_reg,
            &self.branch_eq,
            &self.branch_lt,
            &self.div,
            &self.jal,
            &self.jalr,
            &self.load_store,
            &self.lt_imm,
            &self.lt_reg,
            &self.lui,
            &self.mul,
            &self.mulh,
            &self.shifts_imm,
            &self.shifts_reg,
            &self.program,
            &self.memory,
            &self.merkle,
            &self.poseidon2,
            &self.mem_clock_update,
            &self.reg_clock_update,
        ];
        verifiers.extend(self.preprocessed.verifiers());
        verifiers
    }

    /// Collect relation tracker entries from all components.
    pub fn relation_entries(
        &self,
        trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
    ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
        use stwo_constraint_framework::relation_tracker::add_to_relation_entries;

        let mut entries = vec![];
        entries.extend(add_to_relation_entries(&self.auipc, trace));
        entries.extend(add_to_relation_entries(&self.base_alu_imm, trace));
        entries.extend(add_to_relation_entries(&self.base_alu_reg, trace));
        entries.extend(add_to_relation_entries(&self.branch_eq, trace));
        entries.extend(add_to_relation_entries(&self.branch_lt, trace));
        entries.extend(add_to_relation_entries(&self.div, trace));
        entries.extend(add_to_relation_entries(&self.jal, trace));
        entries.extend(add_to_relation_entries(&self.jalr, trace));
        entries.extend(add_to_relation_entries(&self.load_store, trace));
        entries.extend(add_to_relation_entries(&self.lt_imm, trace));
        entries.extend(add_to_relation_entries(&self.lt_reg, trace));
        entries.extend(add_to_relation_entries(&self.lui, trace));
        entries.extend(add_to_relation_entries(&self.mul, trace));
        entries.extend(add_to_relation_entries(&self.mulh, trace));
        entries.extend(add_to_relation_entries(&self.shifts_imm, trace));
        entries.extend(add_to_relation_entries(&self.shifts_reg, trace));
        entries.extend(add_to_relation_entries(&self.program, trace));
        entries.extend(add_to_relation_entries(&self.memory, trace));
        entries.extend(add_to_relation_entries(&self.merkle, trace));
        entries.extend(add_to_relation_entries(&self.poseidon2, trace));
        entries.extend(add_to_relation_entries(&self.mem_clock_update, trace));
        entries.extend(add_to_relation_entries(&self.reg_clock_update, trace));
        entries.extend(self.preprocessed.relation_entries(trace));
        entries
    }

    /// Collect trace log degree bounds from all components.
    pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
        let mut bounds = vec![
            self.auipc.trace_log_degree_bounds(),
            self.base_alu_imm.trace_log_degree_bounds(),
            self.base_alu_reg.trace_log_degree_bounds(),
            self.branch_eq.trace_log_degree_bounds(),
            self.branch_lt.trace_log_degree_bounds(),
            self.div.trace_log_degree_bounds(),
            self.jal.trace_log_degree_bounds(),
            self.jalr.trace_log_degree_bounds(),
            self.load_store.trace_log_degree_bounds(),
            self.lt_imm.trace_log_degree_bounds(),
            self.lt_reg.trace_log_degree_bounds(),
            self.lui.trace_log_degree_bounds(),
            self.mul.trace_log_degree_bounds(),
            self.mulh.trace_log_degree_bounds(),
            self.shifts_imm.trace_log_degree_bounds(),
            self.shifts_reg.trace_log_degree_bounds(),
            self.program.trace_log_degree_bounds(),
            self.memory.trace_log_degree_bounds(),
            self.merkle.trace_log_degree_bounds(),
            self.poseidon2.trace_log_degree_bounds(),
            self.mem_clock_update.trace_log_degree_bounds(),
            self.reg_clock_update.trace_log_degree_bounds(),
        ];
        bounds.extend(self.preprocessed.trace_log_degree_bounds());
        bounds
    }

    /// Track relations for debugging LogUp imbalances using the original traces.
    ///
    /// Returns a summary showing which relations have mismatched counts.
    pub fn track_relations(
        &self,
        preprocessed_trace: &TraceColumns,
        traces: &Traces,
    ) -> RelationSummary {
        let preprocessed_cpu: Vec<Vec<BaseField>> = preprocessed_trace
            .iter()
            .map(|col| col.values.to_cpu())
            .collect();
        let main_columns = traces.columns_cloned();
        let main_cpu: Vec<Vec<BaseField>> =
            main_columns.iter().map(|col| col.values.to_cpu()).collect();

        let cpu_trace = TreeVec::new(vec![preprocessed_cpu, main_cpu]);
        let trace_refs = TreeVec::new(cpu_trace.iter().map(|tree| tree.iter().collect()).collect());

        let entries = self.relation_entries(&trace_refs);
        RelationSummary::summarize_relations(&entries).cleaned()
    }

    /// Assert constraints on polynomials for all components.
    pub fn assert_constraints_on_polys(traces: &Traces, relations: &Relations) {
        use stwo::core::pcs::TreeVec;
        use stwo::core::poly::circle::CanonicCoset;
        use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
        use tracing::info;

        assert_trace_constraints!(traces, relations, auipc, "auipc");
        assert_trace_constraints!(traces, relations, base_alu_imm, "base_alu_imm");
        assert_trace_constraints!(traces, relations, base_alu_reg, "base_alu_reg");
        assert_trace_constraints!(traces, relations, branch_eq, "branch_eq");
        assert_trace_constraints!(traces, relations, branch_lt, "branch_lt");
        assert_trace_constraints!(traces, relations, div, "div");
        assert_trace_constraints!(traces, relations, jal, "jal");
        assert_trace_constraints!(traces, relations, jalr, "jalr");
        assert_trace_constraints!(traces, relations, load_store, "load_store");
        assert_trace_constraints!(traces, relations, lt_imm, "lt_imm");
        assert_trace_constraints!(traces, relations, lt_reg, "lt_reg");
        assert_trace_constraints!(traces, relations, lui, "lui");
        assert_trace_constraints!(traces, relations, mul, "mul");
        assert_trace_constraints!(traces, relations, mulh, "mulh");
        assert_trace_constraints!(traces, relations, shifts_imm, "shifts_imm");
        assert_trace_constraints!(traces, relations, shifts_reg, "shifts_reg");
        assert_trace_constraints!(traces, relations, program, "program");
        assert_trace_constraints!(traces, relations, memory, "memory");
        assert_trace_constraints!(traces, relations, merkle, "merkle");
        assert_trace_constraints!(traces, relations, poseidon2, "poseidon2");
        assert_trace_constraints!(traces, relations, mem_clock_update, "mem_clock_update");
        assert_trace_constraints!(traces, relations, reg_clock_update, "reg_clock_update");
        preprocessed::Components::assert_constraints_on_polys(&traces.preprocessed, relations);
    }
}

macro_rules! gen_component_interaction {
    ($all_columns:ident, $traces:ident, $relations:ident, $module:ident) => {{
        let (columns, claimed_sum) =
            $module::witness::gen_interaction_trace(&$traces.$module, $relations);
        $all_columns.extend(columns);
        claimed_sum
    }};
}

/// Generate all traces from execution.
pub fn gen_trace(mut tracer: runner::trace::Tracer) -> Traces {
    let mut counters = crate::relations::Counters::new();

    let tracer_program = std::mem::take(&mut tracer.program);
    let tracer_memory = std::mem::take(&mut tracer.memory);
    let tracer_merkle = std::mem::take(&mut tracer.merkle);
    let tracer_poseidon2 = std::mem::take(&mut tracer.poseidon2);
    let tracer_mem_clock_update = std::mem::take(&mut tracer.mem_clock_update);
    let tracer_reg_clock_update = std::mem::take(&mut tracer.reg_clock_update);

    let opcodes = opcodes::gen_trace(tracer, &mut counters);

    let program = tracer_program.into_witness();
    program::witness::register_multiplicities(&program, &mut counters);
    let memory = tracer_memory.into_witness();
    memory::witness::register_multiplicities(&memory, &mut counters);
    let merkle = tracer_merkle.into_witness();
    merkle::witness::register_multiplicities(&merkle, &mut counters);
    let poseidon2 = tracer_poseidon2.into_witness();
    poseidon2::witness::register_multiplicities(&poseidon2, &mut counters);
    let mem_clock_update = tracer_mem_clock_update.into_witness();
    mem_clock_update::witness::register_multiplicities(&mem_clock_update, &mut counters);
    let reg_clock_update = tracer_reg_clock_update.into_witness();
    reg_clock_update::witness::register_multiplicities(&reg_clock_update, &mut counters);

    let preprocessed = preprocessed::Traces::from_counters(counters);

    Traces {
        auipc: opcodes.auipc,
        base_alu_imm: opcodes.base_alu_imm,
        base_alu_reg: opcodes.base_alu_reg,
        branch_eq: opcodes.branch_eq,
        branch_lt: opcodes.branch_lt,
        div: opcodes.div,
        jal: opcodes.jal,
        jalr: opcodes.jalr,
        load_store: opcodes.load_store,
        lt_imm: opcodes.lt_imm,
        lt_reg: opcodes.lt_reg,
        lui: opcodes.lui,
        mul: opcodes.mul,
        mulh: opcodes.mulh,
        shifts_imm: opcodes.shifts_imm,
        shifts_reg: opcodes.shifts_reg,
        program,
        memory,
        merkle,
        poseidon2,
        mem_clock_update,
        reg_clock_update,
        preprocessed,
    }
}

/// Generate all interaction traces.
pub fn gen_interaction_trace(traces: &Traces, relations: &Relations) -> (TraceColumns, ClaimedSum) {
    let mut all_columns = vec![];

    let auipc = gen_component_interaction!(all_columns, traces, relations, auipc);
    let base_alu_imm = gen_component_interaction!(all_columns, traces, relations, base_alu_imm);
    let base_alu_reg = gen_component_interaction!(all_columns, traces, relations, base_alu_reg);
    let branch_eq = gen_component_interaction!(all_columns, traces, relations, branch_eq);
    let branch_lt = gen_component_interaction!(all_columns, traces, relations, branch_lt);
    let div = gen_component_interaction!(all_columns, traces, relations, div);
    let jal = gen_component_interaction!(all_columns, traces, relations, jal);
    let jalr = gen_component_interaction!(all_columns, traces, relations, jalr);
    let load_store = gen_component_interaction!(all_columns, traces, relations, load_store);
    let lt_imm = gen_component_interaction!(all_columns, traces, relations, lt_imm);
    let lt_reg = gen_component_interaction!(all_columns, traces, relations, lt_reg);
    let lui = gen_component_interaction!(all_columns, traces, relations, lui);
    let mul = gen_component_interaction!(all_columns, traces, relations, mul);
    let mulh = gen_component_interaction!(all_columns, traces, relations, mulh);
    let shifts_imm = gen_component_interaction!(all_columns, traces, relations, shifts_imm);
    let shifts_reg = gen_component_interaction!(all_columns, traces, relations, shifts_reg);
    let program = gen_component_interaction!(all_columns, traces, relations, program);
    let memory = gen_component_interaction!(all_columns, traces, relations, memory);
    let merkle = gen_component_interaction!(all_columns, traces, relations, merkle);
    let poseidon2 = gen_component_interaction!(all_columns, traces, relations, poseidon2);
    let mem_clock_update =
        gen_component_interaction!(all_columns, traces, relations, mem_clock_update);
    let reg_clock_update =
        gen_component_interaction!(all_columns, traces, relations, reg_clock_update);

    let (preprocessed_columns, preprocessed) =
        preprocessed::gen_interaction_trace(&traces.preprocessed, relations);
    all_columns.extend(preprocessed_columns);

    let claimed_sum = ClaimedSum {
        auipc,
        base_alu_imm,
        base_alu_reg,
        branch_eq,
        branch_lt,
        div,
        jal,
        jalr,
        load_store,
        lt_imm,
        lt_reg,
        lui,
        mul,
        mulh,
        shifts_imm,
        shifts_reg,
        program,
        memory,
        merkle,
        poseidon2,
        mem_clock_update,
        reg_clock_update,
        preprocessed,
    };

    (all_columns, claimed_sum)
}
