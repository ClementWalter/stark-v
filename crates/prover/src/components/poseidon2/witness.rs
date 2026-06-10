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

    // `wide` and `io` are the last two trace columns (runner's poseidon2 table).
    let wide = &trace[trace.len() - 2].data;
    let io = &trace[trace.len() - 1].data;
    let one = stwo::prover::backend::simd::m31::PackedM31::broadcast(BaseField::from(1));

    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(enabler[i] * (one - io[i])))
        .collect();
    let narrow_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(enabler[i] * (one - wide[i] - io[i])))
        .collect();
    let wide_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(enabler[i] * wide[i]))
        .collect();
    let io_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(enabler[i] * io[i]))
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
    let wide_output_denom = combine!(
        relations.poseidon2,
        [
            &trace[FINAL_STATE_START].data,
            &trace[FINAL_STATE_START + 1].data,
            &trace[FINAL_STATE_START + 2].data,
            &trace[FINAL_STATE_START + 3].data,
            &trace[FINAL_STATE_START + 4].data,
            &trace[FINAL_STATE_START + 5].data,
            &trace[FINAL_STATE_START + 6].data,
            &trace[FINAL_STATE_START + 7].data
        ]
    );

    let io_denom = combine!(
        relations.poseidon2_io,
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
            &trace[16].data,
            &trace[FINAL_STATE_START].data,
            &trace[FINAL_STATE_START + 1].data,
            &trace[FINAL_STATE_START + 2].data,
            &trace[FINAL_STATE_START + 3].data,
            &trace[FINAL_STATE_START + 4].data,
            &trace[FINAL_STATE_START + 5].data,
            &trace[FINAL_STATE_START + 6].data,
            &trace[FINAL_STATE_START + 7].data,
            &trace[FINAL_STATE_START + 8].data,
            &trace[FINAL_STATE_START + 9].data,
            &trace[FINAL_STATE_START + 10].data,
            &trace[FINAL_STATE_START + 11].data,
            &trace[FINAL_STATE_START + 12].data,
            &trace[FINAL_STATE_START + 13].data,
            &trace[FINAL_STATE_START + 14].data,
            &trace[FINAL_STATE_START + 15].data
        ]
    );

    write_pair!(
        &neg_enabler,
        &init_state_denom,
        &narrow_enabler,
        &output_denom,
        interaction_trace
    );
    write_pair!(
        &wide_enabler,
        &wide_output_denom,
        &io_enabler,
        &io_denom,
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
