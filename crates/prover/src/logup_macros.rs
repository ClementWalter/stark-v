//! LogUp interaction trace macros for witness generation and AIR constraints.
//!
//! These macros simplify working with LogUp protocol:
//! - `combine!`: Combine columns into PackedQM31 via LookupElements
//! - `emit_col!`: Write 1/denom fraction (positive contribution)
//! - `consume_col!`: Write -1/denom fraction (negative contribution)
//! - `write_col!`: Write arbitrary num/denom fraction
//! - `write_pair!`: Combine two fractions into one column
//! - `emit_pair!`: Positive combined pair
//! - `consume_pair!`: Negative combined pair
//! - `add_to_relation!`: Add LogUp constraint in AIR

/// Combine columns into PackedQM31 via LookupElements.
///
/// # Arguments
/// * `$relations` - A LookupElements instance
/// * `$cols` - A slice/vec of column iterators (e.g., `[&table.col_a, &table.col_b]`)
///
/// # Returns
/// A `Vec<PackedQM31>` containing the combined values for each SIMD row.
#[macro_export]
macro_rules! combine {
    ($relations:expr, $cols:expr $(,)?) => {{
        let cols = $cols;
        let simd_size = cols[0].len();
        let n_cols = cols.len();

        let mut combined: Vec<stwo::prover::backend::simd::qm31::PackedQM31> =
            Vec::with_capacity(simd_size);

        // Create an iterator over all columns simultaneously
        let mut col_iters: Vec<_> = cols.iter().map(|c| c.iter()).collect();

        for _ in 0..simd_size {
            // Collect one row worth of values by pulling one from each iterator
            let mut packed_m31_values = Vec::with_capacity(n_cols);
            for it in &mut col_iters {
                let v = *it.next().unwrap();
                packed_m31_values.push(unsafe {
                    stwo::prover::backend::simd::m31::PackedM31::from_simd_unchecked(v)
                });
            }
            combined.push($relations.combine(&packed_m31_values));
        }
        combined
    }};
}

/// Write 1/denom fraction to interaction trace (emit/positive contribution).
///
/// # Arguments
/// * `$denom` - Iterator over PackedQM31 denominators
/// * `$interaction_trace` - A mutable LogupTraceGenerator reference
#[macro_export]
macro_rules! emit_col {
    ($denom:expr, $interaction_trace:expr) => {
        use num_traits::One;
        let mut col = $interaction_trace.new_col();
        let one = stwo::prover::backend::simd::qm31::PackedQM31::one();
        for (vec_row, &d) in $denom.iter().enumerate() {
            col.write_frac(vec_row, one, d);
        }
        col.finalize_col();
    };
}

/// Write -1/denom fraction to interaction trace (consume/negative contribution).
///
/// # Arguments
/// * `$denom` - Iterator over PackedQM31 denominators
/// * `$interaction_trace` - A mutable LogupTraceGenerator reference
#[macro_export]
macro_rules! consume_col {
    ($denom:expr, $interaction_trace:expr) => {
        use num_traits::One;
        let mut col = $interaction_trace.new_col();
        let minus_one = -stwo::prover::backend::simd::qm31::PackedQM31::one();
        for (vec_row, &d) in $denom.iter().enumerate() {
            col.write_frac(vec_row, minus_one, d);
        }
        col.finalize_col();
    };
}

/// Write arbitrary num/denom fraction to interaction trace.
///
/// # Arguments
/// * `$numerator` - Iterator over PackedQM31 numerators
/// * `$denom` - Iterator over PackedQM31 denominators
/// * `$interaction_trace` - A mutable LogupTraceGenerator reference
#[macro_export]
macro_rules! write_col {
    ($numerator:expr, $denom:expr, $interaction_trace:expr) => {
        let mut col = $interaction_trace.new_col();
        for (vec_row, (n, d)) in itertools::izip!($numerator, $denom).enumerate() {
            col.write_frac(vec_row, n, d);
        }
        col.finalize_col();
    };
}

/// Combine two fractions into one column: (n0/d0 + n1/d1) = (n0*d1 + n1*d0)/(d0*d1)
///
/// # Arguments
/// * `$numerator_0`, `$denom_0` - First fraction
/// * `$numerator_1`, `$denom_1` - Second fraction
/// * `$interaction_trace` - A mutable LogupTraceGenerator reference
#[macro_export]
macro_rules! write_pair {
    (
        $numerator_0:expr,
        $denom_0:expr,
        $numerator_1:expr,
        $denom_1:expr,
        $interaction_trace:expr
    ) => {{
        let mut col = $interaction_trace.new_col();
        for (vec_row, (n_0, d_0, n_1, d_1)) in
            itertools::izip!($numerator_0, $denom_0, $numerator_1, $denom_1).enumerate()
        {
            let numerator = n_0 * d_1 + n_1 * d_0;
            let denom = d_0 * d_1;
            col.write_frac(vec_row, numerator, denom);
        }
        col.finalize_col();
    }};
}

/// Consume a pair of denominators: write -(d0+d1)/(d0*d1).
///
/// Has two variants:
/// 1. `consume_pair!($interaction_trace; $col1, $col2, ...)` - consume columns in pairs
/// 2. `consume_pair!($denom_0, $denom_1, $interaction_trace)` - consume two specific columns
#[macro_export]
macro_rules! consume_pair {
    // Variant that takes a list of columns to consume in pairs
    ($interaction_trace:expr; $($col:expr),+ $(,)?) => {{
        let secure_columns = vec![$($col),+];
        for [pair0, pair1] in secure_columns.into_iter().array_chunks::<2>() {
            let mut col = $interaction_trace.new_col();
            for (vec_row, (d_0, d_1)) in itertools::izip!(pair0.iter(), pair1.iter()).enumerate() {
                let numerator = *d_0 + *d_1;
                let denom = *d_0 * *d_1;
                col.write_frac(vec_row, -numerator, denom);
            }
            col.finalize_col();
        }
    }};

    // Variant that takes two columns to write in pairs
    ($denom_0:expr, $denom_1:expr, $interaction_trace:expr) => {{
        let mut col = $interaction_trace.new_col();
        for (vec_row, (d_0, d_1)) in itertools::izip!($denom_0, $denom_1).enumerate() {
            let numerator = d_0 + d_1;
            let denom = d_0 * d_1;
            col.write_frac(vec_row, -numerator, denom);
        }
        col.finalize_col();
    }};
}

/// Emit a pair of denominators: write (d0+d1)/(d0*d1).
///
/// # Arguments
/// * `$denom_0`, `$denom_1` - The two denominators
/// * `$interaction_trace` - A mutable LogupTraceGenerator reference
#[macro_export]
macro_rules! emit_pair {
    ($denom_0:expr, $denom_1:expr, $interaction_trace:expr) => {{
        let mut col = $interaction_trace.new_col();
        for (vec_row, (d_0, d_1)) in itertools::izip!($denom_0, $denom_1).enumerate() {
            let numerator = d_0 + d_1;
            let denom = d_0 * d_1;
            col.write_frac(vec_row, numerator, denom);
        }
        col.finalize_col();
    }};
}

/// Add a LogUp relation entry in AIR constraints.
///
/// # Arguments
/// * `$eval` - The evaluator implementing `EvalAtRow`
/// * `$relation` - The relation (LookupElements) to add to
/// * `$numerator` - The multiplier (positive for emit, negative for consume)
/// * `$col...` - The columns that form the relation tuple
///
/// # Example
/// ```ignore
/// // Consume program access
/// add_to_relation!(eval, self.relations.program_access, -enabler.clone(),
///     cols.pc, cols.opcode_id, cols.rd_addr, cols.rs1_addr, cols.rs2_addr);
///
/// // Emit register write
/// add_to_relation!(eval, self.relations.register_access, enabler.clone(),
///     cols.rd_addr, cols.rd_next_0, cols.rd_next_1, cols.rd_next_2, cols.rd_next_3);
/// ```
#[macro_export]
macro_rules! add_to_relation {
    ($eval:expr, $relation:expr, $numerator:expr, $($col:expr),+ $(,)?) => {
        {
        $eval.add_to_relation(stwo_constraint_framework::RelationEntry::new(
            &$relation,
            $numerator.clone(),
            &[$($col.clone()),*],
        ))
        }
    };
}
