//! Tests for load_store component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_load_store_witness_gen_empty() {
    let table = runner::trace::Load_storeTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    assert!(trace.is_empty());
}

#[test]
fn test_load_store_interaction_trace() {
    let table = runner::trace::Load_storeTable::new();
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
macro_rules! test_e2e {
    ($opcode:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $opcode _e2e>]() {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

                let tracer = crate::test_utils::run_test_bin(stringify!($opcode));

                assert!(
                    !tracer.load_store.is_empty(),
                    concat!("Expected ", stringify!($opcode), " trace entries in load_store, got none.")
                );

                let mut counters = crate::relations::Counters::new();
                let trace = witness::gen_trace(tracer.load_store, &mut counters);

                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace");

                let relations = crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    witness::gen_interaction_trace(&trace, &relations);

                let traces = TreeVec::new(vec![
                    vec![],
                    trace,
                    interaction_trace,
                ]);

                let trace_polys = traces.map_cols(|c| c.interpolate());

                let eval = air::Eval {
                    log_size,
                    relations: relations.clone(),
                };

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

// Generate e2e tests for all opcodes in load_store family
test_e2e!(lb);
test_e2e!(lh);
test_e2e!(lw);
test_e2e!(lbu);
test_e2e!(lhu);
test_e2e!(sb);
test_e2e!(sh);
test_e2e!(sw);
