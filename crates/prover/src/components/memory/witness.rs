//! Witness and interaction trace generation for the memory commitment component.

use runner::trace::prover_columns::MemoryColumns;

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

    // Column order matches MemoryColumns.
    let cols = MemoryColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();

    let log_size = trace[0].domain.log_size();
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    let leaf_depth = PackedM31::broadcast(M31::from(MAX_TREE_HEIGHT - 1));
    let one = PackedM31::broadcast(M31::one());
    let two = one + one;
    let three = two + one;

    let rw_as = PackedM31::broadcast(M31::one());
    let rw_as_col = vec![rw_as; simd_size];
    let leaf_depth_col = vec![leaf_depth; simd_size];
    let index_base: Vec<PackedM31> = cols.addr.to_vec();
    let index_base_plus_one: Vec<PackedM31> = (0..simd_size).map(|i| index_base[i] + one).collect();
    let index_base_plus_two: Vec<PackedM31> = (0..simd_size).map(|i| index_base[i] + two).collect();
    let index_base_plus_three: Vec<PackedM31> =
        (0..simd_size).map(|i| index_base[i] + three).collect();

    let pos_mult: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.multiplicity[i]))
        .collect();
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();

    let range_check_8_8_0_denom =
        combine!(relations.range_check_8_8, [&cols.value_0, &cols.value_1]);
    let range_check_8_8_1_denom =
        combine!(relations.range_check_8_8, [&cols.value_2, &cols.value_3]);

    let memory_denom = combine!(
        relations.memory_access,
        [
            &rw_as_col,
            cols.addr,
            cols.clock,
            cols.value_0,
            cols.value_1,
            cols.value_2,
            cols.value_3
        ]
    );
    let merkle_0_denom = combine!(
        relations.merkle,
        [&index_base, &leaf_depth_col, cols.value_0, cols.root]
    );
    let merkle_1_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_one,
            &leaf_depth_col,
            cols.value_1,
            cols.root
        ]
    );
    let merkle_2_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_two,
            &leaf_depth_col,
            cols.value_2,
            cols.root
        ]
    );
    let merkle_3_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_three,
            &leaf_depth_col,
            cols.value_3,
            cols.root
        ]
    );

    write_pair!(
        &neg_enabler,
        &range_check_8_8_0_denom,
        &neg_enabler,
        &range_check_8_8_1_denom,
        interaction_trace
    );

    write_pair!(
        &pos_mult,
        &memory_denom,
        &neg_enabler,
        &merkle_0_denom,
        interaction_trace
    );
    write_pair!(
        &neg_enabler,
        &merkle_1_denom,
        &neg_enabler,
        &merkle_2_denom,
        interaction_trace
    );
    write_col!(&neg_enabler, &merkle_3_denom, interaction_trace);

    interaction_trace.finalize_last()
}

/// Register multiplicities for preprocessed lookups.
pub fn register_multiplicities(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    counters: &mut crate::relations::Counters,
) {
    if trace.is_empty() {
        return;
    }

    let cols = MemoryColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size).map(|i| -cols.enabler[i]).collect();

    // Register range_check_8_8 with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.value_0, cols.value_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.value_2, cols.value_3]);
}
