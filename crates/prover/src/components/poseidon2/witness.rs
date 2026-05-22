//! Witness and interaction trace generation for the Poseidon2 component.

use super::*;

pub fn gen_interaction_trace(
    trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    relations: &Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    if trace.is_empty() {
        return (vec![], QM31::zero());
    }

    let enabler = &trace[0].data;
    let simd_size = enabler.len();
    let log_size = trace[0].domain.log_size();
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(enabler[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(enabler[i]))
        .collect();

    let init_state_denom = combine!(
        relations.poseidon2,
        [
            &trace[1].data,
            &trace[2].data,
            &trace[3].data,
            &trace[4].data,
            &trace[5].data,
            &trace[6].data,
            &trace[7].data,
            &trace[8].data,
            &trace[9].data,
            &trace[10].data,
            &trace[11].data,
            &trace[12].data,
            &trace[13].data,
            &trace[14].data,
            &trace[15].data,
            &trace[16].data
        ]
    );
    let output_denom = combine!(relations.poseidon2, [&trace[FINAL_STATE_START].data]);

    write_pair!(
        &neg_enabler,
        &init_state_denom,
        &pos_enabler,
        &output_denom,
        interaction_trace
    );

    interaction_trace.finalize_last()
}

/// Poseidon2 rows do not request preprocessed lookup multiplicities.
pub fn register_multiplicities(
    _trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    _counters: &mut crate::relations::Counters,
) {
}
