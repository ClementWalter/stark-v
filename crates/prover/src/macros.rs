//! Utility macros for the prover.

/// Helper macro to count identifiers
#[macro_export]
macro_rules! count_idents {
    () => { 0usize };
    ($first:ident $(, $rest:ident)*) => { 1usize + $crate::count_idents!($($rest),*) };
}

/// Macro to generate Relations struct and preprocessed table infrastructure.
///
/// Usage:
/// ```ignore
/// relations! {
///     relations {
///         program_access: addr, clk, value;
///         memory_access: addr, clk, limb_0, limb_1, limb_2, limb_3;
///     }
///     preprocessed {
///         range_check_20: value;
///     }
/// }
/// ```
///
/// Generates:
/// - Wrapper types for each relation that implement `Relation<F, EF>` trait
/// - `Relations` struct with wrapper types for ALL relations (both regular and preprocessed)
/// - `PreProcessedTrace` struct for constant table data
/// - `Counters` struct for multiplicity tracking
#[macro_export]
macro_rules! relations {
    (
        relations {
            $(
                $(#[$rel_meta:meta])*
                $rel_name:ident: $($rel_field:ident),+ $(,)?
            );* $(;)?
        }
        preprocessed {
            $(
                $(#[$prep_meta:meta])*
                $prep_name:ident: $($prep_col:ident),+ $(,)?
            );* $(;)?
        }
    ) => {
        use std::marker::PhantomData;
        use simd::AlignedVec;
        use stwo::core::ColumnVec;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::poly::circle::CanonicCoset;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::backend::simd::column::BaseColumn;
        use stwo::prover::backend::simd::m31::PackedM31;
        use stwo::prover::poly::BitReversedOrder;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

        // ==================== Relation Wrapper Types ====================
        // Each relation gets a wrapper type that implements Relation<F, EF>

        pub mod relation_types {
            // Generate wrapper type for each regular relation
            $(
                $(#[$rel_meta])*
                #[derive(Clone, Debug, PartialEq)]
                pub struct $rel_name(
                    pub stwo_constraint_framework::logup::LookupElements<
                        { $crate::count_idents!($($rel_field),+) }
                    >
                );

                impl $rel_name {
                    pub fn dummy() -> Self {
                        Self(stwo_constraint_framework::logup::LookupElements::dummy())
                    }

                    pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                        Self(stwo_constraint_framework::logup::LookupElements::draw(channel))
                    }
                }

                impl<F: Clone, EF: stwo_constraint_framework::RelationEFTraitBound<F>>
                    stwo_constraint_framework::Relation<F, EF> for $rel_name
                {
                    fn combine(&self, values: &[F]) -> EF {
                        self.0.combine(values)
                    }

                    fn get_name(&self) -> &str {
                        stringify!($rel_name)
                    }

                    fn get_size(&self) -> usize {
                        $crate::count_idents!($($rel_field),+)
                    }
                }
            )*

            // Generate wrapper type for each preprocessed relation
            $(
                $(#[$prep_meta])*
                #[derive(Clone, Debug, PartialEq)]
                pub struct $prep_name(
                    pub stwo_constraint_framework::logup::LookupElements<
                        { $crate::count_idents!($($prep_col),+) }
                    >
                );

                impl $prep_name {
                    pub fn dummy() -> Self {
                        Self(stwo_constraint_framework::logup::LookupElements::dummy())
                    }

                    pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                        Self(stwo_constraint_framework::logup::LookupElements::draw(channel))
                    }
                }

                impl<F: Clone, EF: stwo_constraint_framework::RelationEFTraitBound<F>>
                    stwo_constraint_framework::Relation<F, EF> for $prep_name
                {
                    fn combine(&self, values: &[F]) -> EF {
                        self.0.combine(values)
                    }

                    fn get_name(&self) -> &str {
                        stringify!($prep_name)
                    }

                    fn get_size(&self) -> usize {
                        $crate::count_idents!($($prep_col),+)
                    }
                }
            )*
        }

        // ==================== Relations Struct ====================

        #[derive(Clone)]
        pub struct Relations {
            // Regular relations
            $(
                #[doc = concat!("Relation: (", $(stringify!($rel_field), ", ",)+ ")")]
                pub $rel_name: relation_types::$rel_name,
            )*
            // Preprocessed relations
            $(
                #[doc = concat!("Preprocessed relation: (", $(stringify!($prep_col), ", ",)+ ")")]
                pub $prep_name: relation_types::$prep_name,
            )*
        }

        impl Relations {
            pub fn dummy() -> Self {
                Self {
                    $(
                        $rel_name: relation_types::$rel_name::dummy(),
                    )*
                    $(
                        $prep_name: relation_types::$prep_name::dummy(),
                    )*
                }
            }

            pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                Self {
                    $(
                        $rel_name: relation_types::$rel_name::draw(channel),
                    )*
                    $(
                        $prep_name: relation_types::$prep_name::draw(channel),
                    )*
                }
            }
        }

        // ==================== Preprocessed Tables ====================

        /// Trait for preprocessed table generation.
        pub trait PreprocessedTable {
            const LOG_SIZE: u32;
            /// Compute indices for all 16 SIMD lanes from PackedM31 values.
            /// Each preprocessed table implements the index computation based on its columns.
            fn index(values: &[PackedM31]) -> [u32; 16];
            fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>;
            fn column_ids() -> Vec<PreProcessedColumnId>;
        }

        /// Generic counter for tracking multiplicities.
        pub struct Counter<T: PreprocessedTable> {
            counts: AlignedVec<u32>,
            _marker: PhantomData<T>,
        }

        impl<T: PreprocessedTable> Counter<T> {
            pub fn new() -> Self {
                let size = 1 << T::LOG_SIZE;
                let mut counts = AlignedVec::with_capacity(size);
                counts.resize(size, 0);
                Self {
                    counts,
                    _marker: PhantomData,
                }
            }

            /// Register a single SIMD row with numerator.
            /// Skips lanes where num is 0 (avoids computing potentially invalid indices).
            #[inline]
            pub fn register(&mut self, num: PackedM31, denom: &[PackedM31]) {
                let num_arr = num.to_array();
                // Skip lanes with zero numerator before computing indices
                let indices = T::index(denom);
                for (lane, &n) in num_arr.iter().enumerate() {
                    if n.0 == 0 {
                        continue;
                    }
                    let idx = indices[lane];
                    debug_assert!((idx as usize) < self.counts.len(), "index {idx} out of bounds");
                    self.counts[idx as usize] += n.0;
                }
            }

            /// Register many values at once from column slices with numerators.
            /// Each row across the columns forms one lookup value.
            /// Skips lanes where num is 0 (avoids computing potentially invalid indices).
            ///
            /// Example for range_check_20 (1 column):
            /// ```ignore
            /// counters.range_check_20.register_many(num, &[cols.value]);
            /// ```
            pub fn register_many(&mut self, num: &[PackedM31], denom: &[&[PackedM31]]) {
                if denom.is_empty() {
                    return;
                }
                let len = denom[0].len();
                debug_assert!(num.len() == len, "num length mismatch");
                debug_assert!(denom.iter().all(|c| c.len() == len), "column length mismatch");
                for i in 0..len {
                    let num_arr = num[i].to_array();
                    let values: Vec<PackedM31> = denom.iter().map(|c| c[i]).collect();
                    let indices = T::index(&values);
                    for (lane, &n) in num_arr.iter().enumerate() {
                        if n.0 == 0 {
                            continue;
                        }
                        let idx = indices[lane];
                        debug_assert!((idx as usize) < self.counts.len(), "index {idx} out of bounds");
                        self.counts[idx as usize] += n.0;
                    }
                }
            }

            pub fn into_trace(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let domain = CanonicCoset::new(T::LOG_SIZE).circle_domain();
                let col: BaseColumn = self.counts.into();
                vec![CircleEvaluation::new(domain, col)]
            }
        }

        impl<T: PreprocessedTable> Default for Counter<T> {
            fn default() -> Self {
                Self::new()
            }
        }

        /// Preprocessed trace containing all constant lookup tables.
        pub struct PreProcessedTrace {
            pub trace: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            pub ids: Vec<PreProcessedColumnId>,
        }

        impl PreProcessedTrace {
            pub fn new() -> Self {
                let mut trace = vec![];
                let mut ids = vec![];

                $(
                    trace.extend($crate::preprocessed::$prep_name::Table::gen_columns());
                    ids.extend($crate::preprocessed::$prep_name::Table::column_ids());
                )*

                Self { trace, ids }
            }
        }

        impl Default for PreProcessedTrace {
            fn default() -> Self {
                Self::new()
            }
        }

        /// Aggregate of all multiplicity counters.
        pub struct Counters {
            $(
                #[doc = concat!("Counter for ", stringify!($prep_name), ": (", $(stringify!($prep_col), ", ",)+ ")")]
                pub $prep_name: Counter<$crate::preprocessed::$prep_name::Table>,
            )*
        }

        impl Counters {
            pub fn new() -> Self {
                Self {
                    $(
                        $prep_name: Counter::new(),
                    )*
                }
            }

            pub fn into_traces(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut traces = vec![];
                $(
                    traces.extend(self.$prep_name.into_trace());
                )*
                traces
            }
        }

        impl Default for Counters {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Macro to aggregate all RV32IM opcode components.
///
/// Usage:
/// ```ignore
/// opcode_components! {
///     add, sub, lb, lh, ...
/// }
/// ```
///
/// Generates:
/// - `Traces` struct with one field per opcode (CircleEvaluation columns)
/// - `ClaimedSum` struct with one QM31 field per opcode + `sum()` method
/// - `Components` struct with one air::Component field per opcode
/// - `gen_trace(tracer)` function consuming tracer and calling each component's gen_trace
/// - `gen_interaction_trace(traces, relations)` function aggregating all interaction traces
#[macro_export]
macro_rules! opcode_components {
    ($($opcode:ident),* $(,)?) => {
        use stwo::core::fields::qm31::QM31;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::ColumnVec;
        use stwo::core::air::Component as AirComponent;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::poly::BitReversedOrder;

        /// Trace columns for all components.
        pub struct Traces {
            $(
                pub $opcode: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            )*
        }

        impl Traces {
            /// Returns the maximum log_size across all component traces.
            pub fn max_log_size(&self) -> u32 {
                self.log_sizes().into_iter().max().unwrap_or(4)
            }

            /// Returns log_size for each non-empty component trace.
            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                $(
                    if let Some(first) = self.$opcode.first() {
                        sizes.push(first.domain.log_size());
                    }
                )*
                sizes
            }

            /// Clone all columns into a flattened vec (for commitment).
            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.$opcode.clone());
                )*
                columns
            }

            /// Consume self and return all columns flattened.
            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.$opcode);
                )*
                columns
            }

            /// Print all component tables for debugging.
            /// This is equivalent to `tracer.print_tables(max_rows, max_cols)`.
            pub fn print_tables(&self, max_rows: Option<usize>, max_cols: Option<usize>) {
                use debug_utils::ToTable;
                use stwo::prover::backend::Column;
                debug_utils::set_display_options(max_rows, max_cols);
                $(
                    // If the table is non-empty, print its contents.
                    if !self.$opcode.is_empty() {
                        let table_name = stringify!($opcode);
                        let names = paste::paste! {
                            runner::trace::prover_columns::[<$opcode:camel Columns>]::<()>::NAMES
                        };
                        let table = self.$opcode.to_table_named(names);
                        println!("\n=== {} ({} rows) ===", table_name, self.$opcode.first().unwrap().values.to_cpu().len());
                        println!("{}", table);
                    }
                )*
            }

        }

        /// Claim containing log_size for each component.
        pub struct Claim {
            $(
                pub $opcode: u32,
            )*
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    $(
                        $opcode: traces.$opcode
                            .first()
                            .map(|eval| eval.domain.log_size())
                            .unwrap_or(0),
                    )*
                }
            }
        }

        impl Claim {
            /// Mix claim into the channel.
            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                $(
                    channel.mix_u64(self.$opcode as u64);
                )*
            }
        }

        /// Claimed sums from interaction traces.
        pub struct ClaimedSum {
            $(
                pub $opcode: QM31,
            )*
        }

        impl ClaimedSum {
            /// Sum all claimed values.
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                $(
                    total += self.$opcode;
                )*
                total
            }
        }

        /// AIR components for all opcodes.
        pub struct Components {
            $(
                pub $opcode: $opcode::air::Component,
            )*
        }

        /// Generate all trace columns from tracer.
        /// Consumes the tracer since it's no longer needed after trace generation.
        /// Counters are populated during trace generation for preprocessed lookups.
        pub fn gen_trace(
            tracer: runner::trace::Tracer,
            counters: &mut $crate::relations::Counters,
        ) -> Traces {
            let traces = Traces {
                $(
                    $opcode: tracer.$opcode.into_witness(),
                )*
            };
            $(
                $opcode::witness::register_multiplicities(traces.$opcode.as_slice(), counters);
            )*
            traces
        }

        /// Generate all interaction traces.
        /// Returns interaction trace columns and claimed sums for all components.
        pub fn gen_interaction_trace(
            traces: &Traces,
            relations: &$crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            $(
                let (cols, claimed) = $opcode::witness::gen_interaction_trace(
                    traces.$opcode.as_slice(),
                    relations,
                );
                all_columns.extend(cols);
                let ${concat($opcode, _claimed)} = claimed;
            )*

            let claimed_sum = ClaimedSum {
                $(
                    $opcode: ${concat($opcode, _claimed)},
                )*
            };

            (all_columns, claimed_sum)
        }

        impl Components {
            /// Create all AIR components.
            /// Each component gets its log_size from the claim (minimum 4).
            pub fn new(
                claim: &Claim,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: $crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    $(
                        $opcode: $opcode::air::Component::new(
                            location_allocator,
                            $opcode::air::Eval {
                                log_size: claim.$opcode,
                                relations: relations.clone(),
                            },
                            claimed_sum.$opcode,
                        ),
                    )*
                }
            }

            /// Get all components as trait objects for proving.
            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                vec![ $(&self.$opcode,)* ]
            }

            /// Collect relation tracker entries from all components.
            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                itertools::chain!(
                    $( add_to_relation_entries(&self.$opcode, trace) ),*
                )
                .collect()
            }

            /// Collect trace log degree bounds from all components.
            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                vec![
                    $( self.$opcode.trace_log_degree_bounds(), )*
                ]
            }

            /// Assert constraints on polynomials for all opcode components.
            /// Useful for debugging constraint failures.
            pub fn assert_constraints_on_polys(
                traces: &Traces,
                relations: &$crate::relations::Relations,
            ) {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
                use tracing::info;

                $(
                    if !traces.$opcode.is_empty() {
                        let log_size = traces.$opcode.first()
                            .map(|t| t.domain.log_size())
                            .unwrap_or(0);
                        if log_size > 0 {
                            let (interaction_trace, claimed_sum) =
                                $opcode::witness::gen_interaction_trace(traces.$opcode.as_slice(), relations);
                            let trace_tree = TreeVec::new(vec![
                                vec![], // preprocessed
                                traces.$opcode.clone(),
                                interaction_trace,
                            ]);
                            let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                            let eval = $opcode::air::Eval {
                                log_size,
                                relations: relations.clone(),
                            };
                            info!("Testing {} constraints (log_size={})", stringify!($opcode), log_size);
                            assert_constraints_on_polys(&trace_polys, CanonicCoset::new(log_size),
                                |assert_eval| { eval.evaluate(assert_eval); }, claimed_sum);
                            info!("{} constraints OK", stringify!($opcode));
                        }
                    }
                )*
            }
        }
    };
}

/// Macro to aggregate preprocessed components (multiplicity tracking).
///
/// Usage:
/// ```ignore
/// preprocessed_components! {
///     range_check_20,
///     bitwise,
/// }
/// ```
///
/// Generates for each table:
/// - Module with `air`, `columns`, `witness` submodules
///
/// Generates at aggregate level:
/// - `Traces` struct with one field per preprocessed table (multiplicity columns)
/// - `ClaimedSum` struct with one QM31 field per table + `sum()` method
/// - `Components` struct with one air::Component field per table
/// - `gen_interaction_trace(traces, relations)` function aggregating all interaction traces
#[macro_export]
macro_rules! preprocessed_components {
    ($($table:ident),* $(,)?) => {
        use stwo::core::fields::qm31::QM31;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::ColumnVec;
        use stwo::core::air::Component as AirComponent;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::poly::BitReversedOrder;

        // Generate inner modules for each preprocessed component
        $(
            pub mod $table {
                //! Preprocessed multiplicity component.
                //!
                //! Tracks how many times each value is used by opcode traces.
                //! Provides the "preprocessed side" of the LogUp relation.

                pub mod air {
                    //! AIR component for multiplicity.
                    //!
                    //! Provides the preprocessed side of the LogUp relation:
                    //! Σ (multiplicity[i] / (value[i] - z))

                    use stwo_constraint_framework::{
                        EvalAtRow, FrameworkComponent, FrameworkEval, RelationEntry,
                    };

                    use $crate::preprocessed::PreprocessedTable;
                    use $crate::relations::Relations;

                    pub type Component = FrameworkComponent<Eval>;

                    #[derive(Clone)]
                    pub struct Eval {
                        pub log_size: u32,
                        pub relations: Relations,
                    }

                    impl FrameworkEval for Eval {
                        fn log_size(&self) -> u32 {
                            self.log_size
                        }

                        fn max_constraint_log_degree_bound(&self) -> u32 {
                            self.log_size + 1
                        }

                        fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
                            // Get multiplicity from trace
                            let multiplicity = eval.next_trace_mask();

                            // Get preprocessed column IDs for this table
                            let column_ids = $crate::preprocessed::$table::Table::column_ids();

                            // Get preprocessed column values
                            let preprocessed_cols: Vec<E::F> = column_ids
                                .iter()
                                .map(|id| eval.get_preprocessed_column(id.clone()))
                                .collect();

                            // Add to relation with positive multiplicity (emit side)
                            // Preprocessed tables emit their LogUp contributions
                            eval.add_to_relation(RelationEntry::new(
                                &self.relations.$table,
                                E::EF::from(multiplicity),
                                &preprocessed_cols,
                            ));

                            eval.finalize_logup_in_pairs();

                            eval
                        }
                    }
                }

                pub mod witness {
                    //! Witness generation for multiplicity component.

                    use num_traits::Zero;
                    use stwo::core::ColumnVec;
                    use stwo::core::fields::m31::BaseField;
                    use stwo::core::fields::qm31::QM31;
                    use stwo::prover::backend::simd::SimdBackend;
                    use stwo::prover::backend::simd::qm31::PackedQM31;
                    use stwo::prover::poly::BitReversedOrder;
                    use stwo::prover::poly::circle::CircleEvaluation;
                    use stwo_constraint_framework::LogupTraceGenerator;
                    use stwo_constraint_framework::Relation;

                    use $crate::preprocessed::PreprocessedTable;
                    use $crate::relations::Relations;

                    /// Generate interaction trace for LogUp.
                    ///
                    /// Creates LogUp fractions: multiplicity / (value - z)
                    /// where `value` comes from the preprocessed column.
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

                        let log_size = trace[0].domain.log_size();
                        let mut logup_gen = LogupTraceGenerator::new(log_size);

                        // Get preprocessed columns (constant lookup table values)
                        let preprocessed_cols = $crate::preprocessed::$table::Table::gen_columns();

                        // Get multiplicity from trace (how many times each value was looked up)
                        let multiplicity = &trace[0].values.data;

                        // Convert multiplicity to PackedQM31 for write_col!
                        let multiplicity_qm31: Vec<PackedQM31> = multiplicity
                            .iter()
                            .map(|&m| PackedQM31::from(m))
                            .collect();

                        // Collect preprocessed column data slices for combine!
                        let col_data: Vec<&[stwo::prover::backend::simd::m31::PackedM31]> =
                            preprocessed_cols.iter().map(|c| c.values.data.as_slice()).collect();

                        // Compute denominator by combining preprocessed values with relation
                        let simd_size = col_data[0].len();
                        let mut denom: Vec<PackedQM31> = Vec::with_capacity(simd_size);
                        for row in 0..simd_size {
                            let packed_m31_values: Vec<stwo::prover::backend::simd::m31::PackedM31> =
                                col_data.iter().map(|c| c[row]).collect();
                            denom.push(relations.$table.combine(&packed_m31_values));
                        }

                        // Write multiplicity / denom fraction
                        $crate::write_col!(&multiplicity_qm31, &denom, logup_gen);

                        logup_gen.finalize_last()
                    }
                }
            }
        )*

        /// Trace columns for preprocessed multiplicity components.
        pub struct Traces {
            $(
                pub $table: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            )*
        }

        impl Traces {
            /// Create preprocessed traces from counters.
            /// Each counter's accumulated multiplicities become a trace column.
            pub fn from_counters(counters: $crate::relations::Counters) -> Self {
                Self {
                    $(
                        $table: counters.$table.into_trace(),
                    )*
                }
            }

            /// Returns the maximum log_size across all preprocessed traces.
            pub fn max_log_size(&self) -> u32 {
                self.log_sizes().into_iter().max().unwrap_or(4)
            }

            /// Returns log_size for each non-empty preprocessed trace.
            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                $(
                    if let Some(first) = self.$table.first() {
                        sizes.push(first.domain.log_size());
                    }
                )*
                sizes
            }

            /// Clone all columns into a flattened vec (for commitment).
            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.$table.clone());
                )*
                columns
            }

            /// Consume self and return all columns flattened.
            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.$table);
                )*
                columns
            }

        }

        /// Claim containing log_size for each preprocessed table.
        pub struct Claim {
            $(
                pub $table: u32,
            )*
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    $(
                        $table: traces.$table
                            .first()
                            .map(|eval| eval.domain.log_size())
                            .unwrap_or(0),
                    )*
                }
            }
        }

        impl Claim {
            /// Mix claim into the channel.
            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                $(
                    channel.mix_u64(self.$table as u64);
                )*
            }
        }

        /// Claimed sums from preprocessed interaction traces.
        pub struct ClaimedSum {
            $(
                pub $table: QM31,
            )*
        }

        impl ClaimedSum {
            /// Sum all claimed values.
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                $(
                    total += self.$table;
                )*
                total
            }
        }

        /// AIR components for preprocessed tables.
        pub struct Components {
            $(
                pub $table: $table::air::Component,
            )*
        }

        /// Generate all interaction traces for preprocessed components.
        pub fn gen_interaction_trace(
            traces: &Traces,
            relations: &$crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            $(
                let (cols, claimed) = $table::witness::gen_interaction_trace(
                    &traces.$table,
                    relations,
                );
                all_columns.extend(cols);
                let ${concat($table, _claimed)} = claimed;
            )*

            let claimed_sum = ClaimedSum {
                $(
                    $table: ${concat($table, _claimed)},
                )*
            };

            (all_columns, claimed_sum)
        }

        impl Components {
            /// Create all preprocessed AIR components.
            pub fn new(
                claim: &Claim,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: $crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    $(
                        $table: $table::air::Component::new(
                            location_allocator,
                            $table::air::Eval {
                                log_size: claim.$table,
                                relations: relations.clone(),
                            },
                            claimed_sum.$table,
                        ),
                    )*
                }
            }

            /// Get all components as trait objects for proving.
            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                vec![ $(&self.$table,)* ]
            }

            /// Collect relation tracker entries from all components.
            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                itertools::chain!(
                    $( add_to_relation_entries(&self.$table, trace) ),*
                )
                .collect()
            }

            /// Collect trace log degree bounds from all components.
            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                vec![
                    $( self.$table.trace_log_degree_bounds(), )*
                ]
            }

            /// Assert constraints on polynomials for all preprocessed components.
            /// Useful for debugging constraint failures.
            pub fn assert_constraints_on_polys(
                traces: &Traces,
                relations: &$crate::relations::Relations,
            ) {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
                use tracing::info;
                use $crate::preprocessed::PreprocessedTable;

                $(
                    if !traces.$table.is_empty() {
                        let log_size = traces.$table.first()
                            .map(|t| t.domain.log_size())
                            .unwrap_or(0);
                        if log_size > 0 {
                            let (interaction_trace, claimed_sum) =
                                $table::witness::gen_interaction_trace(&traces.$table, relations);

                            // Get preprocessed columns for this table
                            let preprocessed_cols = $crate::preprocessed::$table::Table::gen_columns();

                            let trace_tree = TreeVec::new(vec![
                                preprocessed_cols,
                                traces.$table.clone(),
                                interaction_trace,
                            ]);
                            let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                            let eval = $table::air::Eval {
                                log_size,
                                relations: relations.clone(),
                            };
                            info!("Testing {} constraints (log_size={})", stringify!($table), log_size);
                            assert_constraints_on_polys(&trace_polys, CanonicCoset::new(log_size),
                                |assert_eval| { eval.evaluate(assert_eval); }, claimed_sum);
                            info!("{} constraints OK", stringify!($table));
                        }
                    }
                )*
            }
        }
    };
}
