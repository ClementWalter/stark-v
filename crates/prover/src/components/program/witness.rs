//! Witness and interaction trace generation for the program component.

use super::columns::ProgramColumns;

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

    let cols = ProgramColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.addr.len();

    let log_size = trace[0].domain.log_size();
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    // Constants
    let leaf_depth = PackedM31::broadcast(M31::from(MAX_TREE_HEIGHT - 1));
    let one = PackedM31::broadcast(M31::one());
    let two = one + one;
    let three = two + one;

    // Compute derived columns
    let leaf_depth_col = vec![leaf_depth; simd_size];
    let index_base: Vec<PackedM31> = cols.addr.to_vec();
    let index_base_plus_one: Vec<PackedM31> = (0..simd_size).map(|i| index_base[i] + one).collect();
    let index_base_plus_two: Vec<PackedM31> = (0..simd_size).map(|i| index_base[i] + two).collect();
    let index_base_plus_three: Vec<PackedM31> =
        (0..simd_size).map(|i| index_base[i] + three).collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: + multiplicity * (addr, value_0, value_1, value_2, value_3)
    let pos_mult: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.multiplicity[i]))
        .collect();

    let program_denom = combine!(
        relations.program_access,
        [
            cols.addr,
            cols.value_0,
            cols.value_1,
            cols.value_2,
            cols.value_3
        ]
    );

    // 2. merkle: -enabler * (index_base, leaf_depth, value_0, root)
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();

    let merkle_0_denom = combine!(
        relations.merkle,
        [&index_base, &leaf_depth_col, cols.value_0, cols.root]
    );

    write_pair!(
        &pos_mult,
        &program_denom,
        &neg_enabler,
        &merkle_0_denom,
        interaction_trace
    );

    // 3. merkle: -enabler * (index_base + 1, leaf_depth, value_1, root)
    let merkle_1_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_one,
            &leaf_depth_col,
            cols.value_1,
            cols.root
        ]
    );

    // 4. merkle: -enabler * (index_base + 2, leaf_depth, value_2, root)
    let merkle_2_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_two,
            &leaf_depth_col,
            cols.value_2,
            cols.root
        ]
    );

    write_pair!(
        &neg_enabler,
        &merkle_1_denom,
        &neg_enabler,
        &merkle_2_denom,
        interaction_trace
    );

    // 5. merkle: -enabler * (index_base + 3, leaf_depth, value_3, root)
    let merkle_3_denom = combine!(
        relations.merkle,
        [
            &index_base_plus_three,
            &leaf_depth_col,
            cols.value_3,
            cols.root
        ]
    );

    write_col!(&neg_enabler, &merkle_3_denom, interaction_trace);

    interaction_trace.finalize_last()
}

/// Program rows do not request preprocessed lookup multiplicities.
pub fn register_multiplicities(
    _trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    _counters: &mut crate::relations::Counters,
) {
}
