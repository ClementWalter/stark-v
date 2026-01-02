//! Component system for RV32IM opcodes and preprocessed tables.
//!
//! This module aggregates all AIR components:
//! - Opcode components (45 RV32IM instructions)
//! - Preprocessed components (range check multiplicity tracking)

pub mod mem_clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod preprocessed;
pub mod program;
pub mod reg_clock_update;

use stwo::core::ColumnVec;
use stwo::core::air::Component;
use stwo::core::channel::Channel;
use stwo::core::channel::MerkleChannel;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::CommitmentSchemeProver;
use stwo::prover::backend::BackendForChannel;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_constraint_framework::relation_tracker::RelationSummary;

use crate::relations::Relations;

/// Aggregate of all trace columns (opcodes + preprocessed multiplicity).
pub struct Traces {
    /// Opcode traces (45 RV32IM instructions).
    pub opcodes: opcodes::Traces,
    /// Program trace rows.
    pub program: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Memory commitment trace rows.
    pub memory: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Merkle node trace rows.
    pub merkle: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Poseidon2 input trace rows.
    pub poseidon2: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Memory clock update trace rows.
    pub mem_clock_update: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Register clock update trace rows.
    pub reg_clock_update: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    /// Preprocessed multiplicity traces.
    pub preprocessed: preprocessed::Traces,
}

impl Traces {
    /// Returns the maximum log_size across all traces.
    pub fn max_log_size(&self) -> u32 {
        self.log_sizes().into_iter().max().unwrap_or(4)
    }

    /// Returns all log_sizes from both opcodes and preprocessed traces.
    pub fn log_sizes(&self) -> Vec<u32> {
        let mut sizes = self.opcodes.log_sizes();
        if let Some(first) = self.program.first() {
            sizes.push(first.domain.log_size());
        }
        if let Some(first) = self.memory.first() {
            sizes.push(first.domain.log_size());
        }
        if let Some(first) = self.merkle.first() {
            sizes.push(first.domain.log_size());
        }
        if let Some(first) = self.poseidon2.first() {
            sizes.push(first.domain.log_size());
        }
        if let Some(first) = self.mem_clock_update.first() {
            sizes.push(first.domain.log_size());
        }
        if let Some(first) = self.reg_clock_update.first() {
            sizes.push(first.domain.log_size());
        }
        sizes.extend(self.preprocessed.log_sizes());
        sizes
    }

    /// Clone all columns into a flattened vec (for commitment).
    pub fn columns_cloned(
        &self,
    ) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let mut columns = self.opcodes.columns_cloned();
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

/// Claim containing log_size for each component.
pub struct Claim {
    /// Opcode claims (log_size per instruction).
    pub opcodes: opcodes::Claim,
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
    /// Preprocessed claims (log_size per table).
    pub preprocessed: preprocessed::Claim,
}

impl From<&Traces> for Claim {
    fn from(traces: &Traces) -> Self {
        Self {
            opcodes: (&traces.opcodes).into(),
            program: traces
                .program
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            memory: traces
                .memory
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            merkle: traces
                .merkle
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            poseidon2: traces
                .poseidon2
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            mem_clock_update: traces
                .mem_clock_update
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            reg_clock_update: traces
                .reg_clock_update
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
            preprocessed: (&traces.preprocessed).into(),
        }
    }
}

impl Claim {
    /// Mix claim into the channel.
    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        channel.mix_u64(self.program as u64);
        channel.mix_u64(self.memory as u64);
        channel.mix_u64(self.merkle as u64);
        channel.mix_u64(self.poseidon2 as u64);
        channel.mix_u64(self.mem_clock_update as u64);
        channel.mix_u64(self.reg_clock_update as u64);
        self.preprocessed.mix_into(channel);
    }
}

/// Aggregate of all claimed sums from interaction traces.
pub struct ClaimedSum {
    /// Claimed sums from opcode components.
    pub opcodes: opcodes::ClaimedSum,
    /// Claimed sum for program component.
    pub program: QM31,
    /// Claimed sum for memory component.
    pub memory: QM31,
    /// Claimed sum for merkle component.
    pub merkle: QM31,
    /// Claimed sum for poseidon2 component.
    pub poseidon2: QM31,
    /// Claimed sum for clock update component.
    pub mem_clock_update: QM31,
    /// Claimed sum for register clock update component.
    pub reg_clock_update: QM31,
    /// Claimed sums from preprocessed components.
    pub preprocessed: preprocessed::ClaimedSum,
}

impl ClaimedSum {
    /// Sum all claimed values (opcodes + preprocessed).
    pub fn total(&self) -> QM31 {
        self.opcodes.sum()
            + self.program
            + self.memory
            + self.merkle
            + self.poseidon2
            + self.mem_clock_update
            + self.reg_clock_update
            + self.preprocessed.sum()
    }
}

/// Aggregate of all AIR components.
pub struct Components {
    /// Opcode components (45 RV32IM instructions).
    pub opcodes: opcodes::Components,
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
    /// Preprocessed components (multiplicity tracking).
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
        let opcodes = opcodes::Components::new(
            &claim.opcodes,
            location_allocator,
            relations.clone(),
            &claimed_sum.opcodes,
        );

        let program = program::air::Component::new(
            location_allocator,
            program::air::Eval {
                log_size: claim.program,
                relations: relations.clone(),
            },
            claimed_sum.program,
        );

        let memory = memory::air::Component::new(
            location_allocator,
            memory::air::Eval {
                log_size: claim.memory,
                relations: relations.clone(),
            },
            claimed_sum.memory,
        );

        let merkle = merkle::air::Component::new(
            location_allocator,
            merkle::air::Eval {
                log_size: claim.merkle,
                relations: relations.clone(),
            },
            claimed_sum.merkle,
        );

        let poseidon2 = poseidon2::air::Component::new(
            location_allocator,
            poseidon2::air::Eval {
                log_size: claim.poseidon2,
                relations: relations.clone(),
            },
            claimed_sum.poseidon2,
        );

        let mem_clock_update = mem_clock_update::air::Component::new(
            location_allocator,
            mem_clock_update::air::Eval {
                log_size: claim.mem_clock_update,
                relations: relations.clone(),
            },
            claimed_sum.mem_clock_update,
        );

        let reg_clock_update = reg_clock_update::air::Component::new(
            location_allocator,
            reg_clock_update::air::Eval {
                log_size: claim.reg_clock_update,
                relations: relations.clone(),
            },
            claimed_sum.reg_clock_update,
        );
        // Create preprocessed components
        let preprocessed = preprocessed::Components::new(
            &claim.preprocessed,
            location_allocator,
            relations,
            &claimed_sum.preprocessed,
        );

        Self {
            opcodes,
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
        let mut provers = self.opcodes.provers();
        provers.push(&self.program);
        provers.push(&self.memory);
        provers.push(&self.merkle);
        provers.push(&self.poseidon2);
        provers.push(&self.mem_clock_update);
        provers.push(&self.reg_clock_update);
        provers.extend(self.preprocessed.provers());
        provers
    }

    /// Collect relation tracker entries from all components.
    pub fn relation_entries(
        &self,
        trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
    ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
        use stwo_constraint_framework::relation_tracker::add_to_relation_entries;

        let mut entries = self.opcodes.relation_entries(trace);
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
        let mut bounds = self.opcodes.trace_log_degree_bounds();
        bounds.push(self.program.trace_log_degree_bounds());
        bounds.push(self.memory.trace_log_degree_bounds());
        bounds.push(self.merkle.trace_log_degree_bounds());
        bounds.push(self.poseidon2.trace_log_degree_bounds());
        bounds.push(self.mem_clock_update.trace_log_degree_bounds());
        bounds.push(self.reg_clock_update.trace_log_degree_bounds());
        bounds.extend(self.preprocessed.trace_log_degree_bounds());
        bounds
    }

    /// Track relations for debugging LogUp imbalances.
    ///
    /// Returns a summary showing which relations have mismatched counts.
    pub fn track_relations<MC: MerkleChannel>(
        &self,
        commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    ) -> RelationSummary
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let evals = commitment_scheme
            .trace()
            .polys
            .map(|tree| tree.iter().map(|poly| poly.evals.to_cpu().values).collect());
        let evals = &evals.as_ref();
        let trace = &evals.into();

        let entries = self.relation_entries(trace);
        RelationSummary::summarize_relations(&entries).cleaned()
    }

    /// Assert constraints on polynomials for all components (opcodes + preprocessed).
    /// Useful for debugging constraint failures.
    pub fn assert_constraints_on_polys(traces: &Traces, relations: &Relations) {
        use stwo::core::pcs::TreeVec;
        use stwo::core::poly::circle::CanonicCoset;
        use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
        use tracing::info;

        macro_rules! assert_trace_constraints {
            ($module:ident, $label:literal) => {
                if !traces.$module.is_empty() {
                    let log_size = traces
                        .$module
                        .first()
                        .map(|t| t.domain.log_size())
                        .unwrap_or(0);
                    if log_size > 0 {
                        let (interaction_trace, claimed_sum) =
                            $module::witness::gen_interaction_trace(&traces.$module, relations);
                        let trace_tree =
                            TreeVec::new(vec![vec![], traces.$module.clone(), interaction_trace]);
                        let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                        let eval = $module::air::Eval {
                            log_size,
                            relations: relations.clone(),
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

        opcodes::Components::assert_constraints_on_polys(&traces.opcodes, relations);

        assert_trace_constraints!(program, "program");
        assert_trace_constraints!(memory, "memory");
        assert_trace_constraints!(merkle, "merkle");
        assert_trace_constraints!(poseidon2, "poseidon2");
        assert_trace_constraints!(mem_clock_update, "mem_clock_update");
        assert_trace_constraints!(reg_clock_update, "reg_clock_update");

        preprocessed::Components::assert_constraints_on_polys(&traces.preprocessed, relations);
    }
}

/// Generate all traces from execution.
///
/// This is the main entry point for trace generation:
/// 1. Creates counters for preprocessed multiplicity tracking
/// 2. Generates opcode traces (populates counters during generation)
/// 3. Converts counters to preprocessed multiplicity traces
pub fn gen_trace(mut tracer: runner::trace::Tracer) -> Traces {
    // Create counters for preprocessed multiplicity tracking
    let mut counters = crate::relations::Counters::new();

    let tracer_program = std::mem::take(&mut tracer.program);
    let tracer_memory = std::mem::take(&mut tracer.memory);
    let tracer_merkle = std::mem::take(&mut tracer.merkle);
    let tracer_poseidon2 = std::mem::take(&mut tracer.poseidon2);
    let tracer_mem_clock_update = std::mem::take(&mut tracer.mem_clk_update);
    let tracer_reg_clock_update = std::mem::take(&mut tracer.reg_clk_update);

    // Generate opcode traces (populates counters during generation)
    let opcodes = opcodes::gen_trace(tracer, &mut counters);

    let program = tracer_program.into_witness();
    let memory = tracer_memory.into_witness();
    memory::witness::register_multiplicities(&memory, &mut counters);
    let merkle = tracer_merkle.into_witness();
    let poseidon2 = tracer_poseidon2.into_witness();
    let mem_clock_update = tracer_mem_clock_update.into_witness();
    let reg_clock_update = tracer_reg_clock_update.into_witness();

    // Convert counters to preprocessed multiplicity traces
    counters.print_counters(None, None);
    let preprocessed = preprocessed::Traces::from_counters(counters);
    preprocessed.print_tables(None, None);

    Traces {
        opcodes,
        program,
        memory,
        merkle,
        poseidon2,
        mem_clock_update,
        reg_clock_update,
        preprocessed,
    }
}

/// Generate all interaction traces (opcodes + preprocessed).
pub fn gen_interaction_trace(
    traces: &Traces,
    relations: &Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ClaimedSum,
) {
    // Generate opcode interaction traces
    let (mut all_columns, opcodes_claimed) =
        opcodes::gen_interaction_trace(&traces.opcodes, relations);

    let (program_columns, program_claimed) =
        program::witness::gen_interaction_trace(&traces.program, relations);
    let (memory_columns, memory_claimed) =
        memory::witness::gen_interaction_trace(&traces.memory, relations);
    let (merkle_columns, merkle_claimed) =
        merkle::witness::gen_interaction_trace(&traces.merkle, relations);
    let (poseidon2_columns, poseidon2_claimed) =
        poseidon2::witness::gen_interaction_trace(&traces.poseidon2, relations);
    let (mem_clock_update_columns, mem_clock_update_claimed) =
        mem_clock_update::witness::gen_interaction_trace(&traces.mem_clock_update, relations);
    let (reg_clock_update_columns, reg_clock_update_claimed) =
        reg_clock_update::witness::gen_interaction_trace(&traces.reg_clock_update, relations);

    all_columns.extend(program_columns);
    all_columns.extend(memory_columns);
    all_columns.extend(merkle_columns);
    all_columns.extend(poseidon2_columns);
    all_columns.extend(mem_clock_update_columns);
    all_columns.extend(reg_clock_update_columns);

    // Generate preprocessed interaction traces
    let (preprocessed_columns, preprocessed_claimed) =
        preprocessed::gen_interaction_trace(&traces.preprocessed, relations);
    all_columns.extend(preprocessed_columns);

    let claimed_sum = ClaimedSum {
        opcodes: opcodes_claimed,
        program: program_claimed,
        memory: memory_claimed,
        merkle: merkle_claimed,
        poseidon2: poseidon2_claimed,
        mem_clock_update: mem_clock_update_claimed,
        reg_clock_update: reg_clock_update_claimed,
        preprocessed: preprocessed_claimed,
    };

    (all_columns, claimed_sum)
}
