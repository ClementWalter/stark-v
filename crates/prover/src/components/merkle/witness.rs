//! Witness and interaction trace generation for the Merkle component.

use runner::trace::prover_columns::MerkleColumns;

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

    // Column order matches MerkleColumns.
    let cols = MerkleColumns::from_iter(trace.iter().map(|eval| &eval.values.data));

    let simd_size = cols.enabler.len();
    let log_size = trace[0].domain.log_size();
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    let one = PackedM31::broadcast(M31::one());
    let inv2 = PackedM31::broadcast(M31::inverse(&M31::from(2)));

    let index_plus_one: Vec<PackedM31> = (0..simd_size).map(|i| cols.index[i] + one).collect();
    let index_div2: Vec<PackedM31> = (0..simd_size).map(|i| cols.index[i] * inv2).collect();
    let depth_minus_one: Vec<PackedM31> = (0..simd_size).map(|i| cols.depth[i] - one).collect();

    let left_mult: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.lhs_mult[i]))
        .collect();
    let right_mult: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.rhs_mult[i]))
        .collect();
    let neg_cur_mult: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.cur_mult[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();

    let left_denom = combine!(
        relations.merkle,
        [cols.index, cols.depth, cols.lhs, cols.root]
    );
    let right_denom = combine!(
        relations.merkle,
        [&index_plus_one, cols.depth, cols.rhs, cols.root]
    );
    let cur_denom = combine!(
        relations.merkle,
        [&index_div2, &depth_minus_one, cols.cur, cols.root]
    );
    let poseidon_in_denom = combine!(relations.poseidon2, [cols.lhs, cols.rhs]);
    let poseidon_out_denom = combine!(relations.poseidon2, [cols.cur]);

    write_pair!(
        &left_mult,
        &left_denom,
        &right_mult,
        &right_denom,
        interaction_trace
    );
    write_pair!(
        &neg_cur_mult,
        &cur_denom,
        &pos_enabler,
        &poseidon_in_denom,
        interaction_trace
    );
    write_col!(&neg_enabler, &poseidon_out_denom, interaction_trace);

    interaction_trace.finalize_last()
}

/// Merkle rows do not request preprocessed lookup multiplicities.
pub fn register_multiplicities(
    _trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    _counters: &mut crate::relations::Counters,
) {
}
