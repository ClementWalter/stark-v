//! Witness and interaction trace generation for the memory clock update component.

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

    // Column order matches MemClockUpdateColumns.
    let enabler = &trace[0].data;
    let addr = &trace[1].data;
    let clock_prev = &trace[2].data;
    let value_0 = &trace[3].data;
    let value_1 = &trace[4].data;
    let value_2 = &trace[5].data;
    let value_3 = &trace[6].data;

    let diff = PackedM31::broadcast(M31::from(DEFAULT_MAX_CLOCK_DIFF));

    let simd_size = enabler.len();
    let log_size = trace[0].domain.log_size();
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    let rw_as = PackedM31::broadcast(M31::one());
    let rw_as_col = vec![rw_as; simd_size];
    let clock_prev_plus_diff: Vec<PackedM31> =
        (0..simd_size).map(|i| clock_prev[i] + diff).collect();

    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(enabler[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(enabler[i]))
        .collect();

    let prev_denom = combine!(
        relations.memory_access,
        [
            &rw_as_col, addr, clock_prev, value_0, value_1, value_2, value_3
        ]
    );
    let next_denom = combine!(
        relations.memory_access,
        [
            &rw_as_col,
            addr,
            &clock_prev_plus_diff,
            value_0,
            value_1,
            value_2,
            value_3
        ]
    );

    write_pair!(
        &neg_enabler,
        &prev_denom,
        &pos_enabler,
        &next_denom,
        interaction_trace
    );

    interaction_trace.finalize_last()
}

/// Memory clock update rows do not request preprocessed lookup multiplicities.
pub fn register_multiplicities(
    _trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    _counters: &mut crate::relations::Counters,
) {
}
