//! Component system for RV32IM opcodes and preprocessed tables.
//!
//! This module aggregates all AIR components:
//! - Opcode components (45 RV32IM instructions)
//! - Preprocessed components (range check multiplicity tracking)

pub mod opcodes;
pub mod preprocessed;

use stwo::core::ColumnVec;
use stwo::core::channel::Channel;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::TraceLocationAllocator;

use crate::relations::Relations;

/// Aggregate of all trace columns (opcodes + preprocessed multiplicity).
pub struct Traces {
    /// Opcode traces (45 RV32IM instructions).
    pub opcodes: opcodes::Traces,
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
        sizes.extend(self.preprocessed.log_sizes());
        sizes
    }

    /// Clone all columns into a flattened vec (for commitment).
    pub fn columns_cloned(
        &self,
    ) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let mut columns = self.opcodes.columns_cloned();
        columns.extend(self.preprocessed.columns_cloned());
        columns
    }
}

/// Claim containing log_size for each component.
pub struct Claim {
    /// Opcode claims (log_size per instruction).
    pub opcodes: opcodes::Claim,
    /// Preprocessed claims (log_size per table).
    pub preprocessed: preprocessed::Claim,
}

impl From<&Traces> for Claim {
    fn from(traces: &Traces) -> Self {
        Self {
            opcodes: (&traces.opcodes).into(),
            preprocessed: (&traces.preprocessed).into(),
        }
    }
}

impl Claim {
    /// Mix claim into the channel.
    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.opcodes.mix_into(channel);
        self.preprocessed.mix_into(channel);
    }
}

/// Aggregate of all claimed sums from interaction traces.
pub struct ClaimedSum {
    /// Claimed sums from opcode components.
    pub opcodes: opcodes::ClaimedSum,
    /// Claimed sums from preprocessed components.
    pub preprocessed: preprocessed::ClaimedSum,
}

impl ClaimedSum {
    /// Sum all claimed values (opcodes + preprocessed).
    pub fn total(&self) -> QM31 {
        self.opcodes.sum() + self.preprocessed.sum()
    }
}

/// Aggregate of all AIR components.
pub struct Components {
    /// Opcode components (45 RV32IM instructions).
    pub opcodes: opcodes::Components,
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
        // Create opcode components first
        let opcodes = opcodes::Components::new(
            &claim.opcodes,
            location_allocator,
            relations.clone(),
            &claimed_sum.opcodes,
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
            preprocessed,
        }
    }

    /// Get all components as trait objects for proving.
    pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
        let mut provers = self.opcodes.provers();
        provers.extend(self.preprocessed.provers());
        provers
    }

    /// Collect relation tracker entries from all components.
    pub fn relation_entries(
        &self,
        trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
    ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
        let mut entries = self.opcodes.relation_entries(trace);
        entries.extend(self.preprocessed.relation_entries(trace));
        entries
    }

    /// Collect trace log degree bounds from all components.
    pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
        let mut bounds = self.opcodes.trace_log_degree_bounds();
        bounds.extend(self.preprocessed.trace_log_degree_bounds());
        bounds
    }

    /// Assert constraints on polynomials for all components (opcodes + preprocessed).
    /// Useful for debugging constraint failures.
    pub fn assert_constraints_on_polys(traces: &Traces, relations: &Relations) {
        opcodes::Components::assert_constraints_on_polys(&traces.opcodes, relations);
        preprocessed::Components::assert_constraints_on_polys(&traces.preprocessed, relations);
    }
}

/// Generate all traces from execution.
///
/// This is the main entry point for trace generation:
/// 1. Creates counters for preprocessed multiplicity tracking
/// 2. Generates opcode traces (populates counters during generation)
/// 3. Converts counters to preprocessed multiplicity traces
///
/// Consumes the tracer since it's no longer needed after trace generation.
pub fn gen_trace(tracer: runner::trace::Tracer) -> Traces {
    // Create counters for preprocessed multiplicity tracking
    let mut counters = crate::relations::Counters::new();

    // Generate opcode traces (populates counters during generation)
    let opcodes = opcodes::gen_trace(tracer, &mut counters);

    // Convert counters to preprocessed multiplicity traces
    let preprocessed = preprocessed::Traces::from_counters(counters);

    Traces {
        opcodes,
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

    // Generate preprocessed interaction traces
    let (preprocessed_columns, preprocessed_claimed) =
        preprocessed::gen_interaction_trace(&traces.preprocessed, relations);
    all_columns.extend(preprocessed_columns);

    let claimed_sum = ClaimedSum {
        opcodes: opcodes_claimed,
        preprocessed: preprocessed_claimed,
    };

    (all_columns, claimed_sum)
}
