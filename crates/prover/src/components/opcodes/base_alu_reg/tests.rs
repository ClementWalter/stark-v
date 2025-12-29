//! Tests for base_alu_reg component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_base_alu_reg_witness_gen_empty() {
    let table = runner::trace::Base_alu_regTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    assert!(trace.is_empty());
}

#[test]
fn test_base_alu_reg_interaction_trace() {
    let table = runner::trace::Base_alu_regTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);
    assert!(interaction_trace.is_empty());
    assert_eq!(claimed_sum, QM31::zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

/// Macro to generate e2e test for a single opcode in this family.
///
/// Usage: `test_e2e!(add);`
macro_rules! test_e2e {
    ($opcode:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $opcode _e2e>]() {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

                let tracer = crate::test_utils::run_test_bin(stringify!($opcode));

                // Verify trace was captured
                assert!(
                    !tracer.base_alu_reg.is_empty(),
                    concat!("Expected ", stringify!($opcode), " trace entries in base_alu_reg, got none. ",
                           "Make sure the binary executes the instruction.")
                );

                // Generate traces
                let mut counters = crate::relations::Counters::new();
                let trace = witness::gen_trace(tracer.base_alu_reg, &mut counters);

                // Get log_size from trace
                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace");

                // Generate interaction trace
                let relations = crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    witness::gen_interaction_trace(&trace, &relations);

                // Build TreeVec: [preprocessed, main, interaction]
                let traces = TreeVec::new(vec![
                    vec![],  // No preprocessed
                    trace,
                    interaction_trace,
                ]);

                // Convert to polynomials
                let trace_polys = traces.map_cols(|c| c.interpolate());

                // Create the evaluator
                let eval = air::Eval {
                    log_size,
                    relations: relations.clone(),
                };

                // Assert constraints
                assert_constraints_on_polys(
                    &trace_polys,
                    CanonicCoset::new(log_size),
                    |assert_eval| {
                        eval.evaluate(assert_eval);
                    },
                    claimed_sum,
                );
            }
        }
    };
}

// Generate e2e tests for all opcodes in base_alu_reg family
test_e2e!(add);
test_e2e!(sub);
test_e2e!(xor);
test_e2e!(or);
test_e2e!(and);
