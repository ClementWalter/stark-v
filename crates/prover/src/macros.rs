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
/// - `Relations` struct with `LookupElements<N>` for ALL relations (both regular and preprocessed)
/// - `PreProcessedTrace` struct for constant table data
/// - `Counters` struct for multiplicity tracking
#[macro_export]
macro_rules! relations {
    (
        relations {
            $(
                $rel_name:ident: $($rel_field:ident),+ $(,)?
            );* $(;)?
        }
        preprocessed {
            $(
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
        use stwo::prover::poly::BitReversedOrder;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

        // ==================== Relations ====================

        #[derive(Clone)]
        pub struct Relations {
            // Regular relations
            $(
                #[doc = concat!("Relation: (", $(stringify!($rel_field), ", ",)+ ")")]
                pub $rel_name: stwo_constraint_framework::logup::LookupElements<
                    { $crate::count_idents!($($rel_field),+) }
                >,
            )*
            // Preprocessed relations
            $(
                #[doc = concat!("Preprocessed relation: (", $(stringify!($prep_col), ", ",)+ ")")]
                pub $prep_name: stwo_constraint_framework::logup::LookupElements<
                    { $crate::count_idents!($($prep_col),+) }
                >,
            )*
        }

        impl Relations {
            pub fn dummy() -> Self {
                Self {
                    $(
                        $rel_name: stwo_constraint_framework::logup::LookupElements::dummy(),
                    )*
                    $(
                        $prep_name: stwo_constraint_framework::logup::LookupElements::dummy(),
                    )*
                }
            }

            pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                Self {
                    $(
                        $rel_name: stwo_constraint_framework::logup::LookupElements::draw(channel),
                    )*
                    $(
                        $prep_name: stwo_constraint_framework::logup::LookupElements::draw(channel),
                    )*
                }
            }
        }

        // ==================== Preprocessed Tables ====================

        /// Trait for preprocessed table generation.
        pub trait PreprocessedTable<const N: usize> {
            const LOG_SIZE: u32;
            fn index(values: [u32; N]) -> u32;
            fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>;
            fn column_ids() -> Vec<PreProcessedColumnId>;
        }

        /// Generic counter for tracking multiplicities.
        pub struct Counter<T: PreprocessedTable<N>, const N: usize> {
            counts: AlignedVec<u32>,
            _marker: PhantomData<T>,
        }

        impl<T: PreprocessedTable<N>, const N: usize> Counter<T, N> {
            pub fn new() -> Self {
                let size = 1 << T::LOG_SIZE;
                let mut counts = AlignedVec::with_capacity(size);
                counts.resize(size, 0);
                Self {
                    counts,
                    _marker: PhantomData,
                }
            }

            #[inline]
            pub fn register(&mut self, values: [u32; N]) {
                let idx = T::index(values) as usize;
                debug_assert!(idx < self.counts.len(), "index {idx} out of bounds");
                self.counts[idx] += 1;
            }

            /// Register many values at once from column slices.
            /// Each row across the columns forms one lookup value.
            ///
            /// Example for N=1 (range_check_20):
            /// ```ignore
            /// counters.range_check_20.register_many([&trace.value]);
            /// ```
            pub fn register_many(&mut self, columns: [&[u32]; N]) {
                let len = columns[0].len();
                debug_assert!(columns.iter().all(|c| c.len() == len), "column length mismatch");
                for i in 0..len {
                    let values: [u32; N] = std::array::from_fn(|j| columns[j][i]);
                    let idx = T::index(values) as usize;
                    debug_assert!(idx < self.counts.len(), "index {idx} out of bounds");
                    self.counts[idx] += 1;
                }
            }

            /// Add counts from a vector (element-wise merge).
            /// Used when the tracer has already accumulated counts in an AlignedVec.
            pub fn add_counts(&mut self, counts: &[u32]) {
                assert_eq!(
                    counts.len(),
                    self.counts.len(),
                    "counts length mismatch: expected {}, got {}",
                    self.counts.len(),
                    counts.len()
                );
                for (dest, src) in self.counts.iter_mut().zip(counts.iter()) {
                    *dest += src;
                }
            }

            pub fn into_trace(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let domain = CanonicCoset::new(T::LOG_SIZE).circle_domain();
                let col: BaseColumn = self.counts.into();
                vec![CircleEvaluation::new(domain, col)]
            }
        }

        impl<T: PreprocessedTable<N>, const N: usize> Default for Counter<T, N> {
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
                    trace.extend(
                        <$crate::preprocessed::$prep_name::Table<{ $crate::count_idents!($($prep_col),+) }>
                            as PreprocessedTable<{ $crate::count_idents!($($prep_col),+) }>>::gen_columns()
                    );
                    ids.extend(
                        <$crate::preprocessed::$prep_name::Table<{ $crate::count_idents!($($prep_col),+) }>
                            as PreprocessedTable<{ $crate::count_idents!($($prep_col),+) }>>::column_ids()
                    );
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
                pub $prep_name: Counter<
                    $crate::preprocessed::$prep_name::Table<{ $crate::count_idents!($($prep_col),+) }>,
                    { $crate::count_idents!($($prep_col),+) }
                >,
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
        /// Takes `&Tracer` to keep it alive for interaction trace generation.
        /// Counters are populated during trace generation for preprocessed lookups.
        pub fn gen_trace(
            tracer: &runner::trace::Tracer,
            counters: &mut $crate::relations::Counters,
        ) -> Traces {
            Traces {
                $(
                    $opcode: tracer.$opcode.to_witness(counters),
                )*
            }
        }

        /// Generate all interaction traces.
        /// Returns interaction trace columns and claimed sums for all components.
        pub fn gen_interaction_trace(
            tracer: &runner::trace::Tracer,
            relations: &$crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            $(
                let (cols, claimed) = $opcode::witness::gen_interaction_trace(
                    &tracer.$opcode,
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
                tracer: &runner::trace::Tracer,
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
                                $opcode::witness::gen_interaction_trace(&tracer.$opcode, relations);
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

                pub mod columns {
                    //! Column definitions for multiplicity component.

                    use stwo_constraint_framework::EvalAtRow;

                    /// Number of trace columns for this component.
                    pub const N_COLUMNS: usize = 1;

                    /// Column offsets.
                    pub const MULTIPLICITY: usize = 0;

                    /// Columns for multiplicity tracking.
                    pub struct Columns<E: EvalAtRow> {
                        pub multiplicity: E::F,
                    }

                    impl<E: EvalAtRow> Columns<E> {
                        pub fn from_eval(eval: &mut E) -> Self {
                            Self {
                                multiplicity: eval.next_trace_mask(),
                            }
                        }
                    }
                }

                pub mod air {
                    //! AIR component for multiplicity.
                    //!
                    //! Provides the preprocessed side of the LogUp relation:
                    //! Σ (multiplicity[i] / (value[i] - z))

                    use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

                    use super::columns::Columns;
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
                            let cols = Columns::from_eval(&mut eval);

                            // TODO: Add LogUp constraint
                            // For now, dummy constraint (multiplicity - multiplicity = 0)
                            eval.add_constraint(cols.multiplicity.clone() - cols.multiplicity.clone());

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
                    use stwo::prover::poly::BitReversedOrder;
                    use stwo::prover::poly::circle::CircleEvaluation;

                    use $crate::relations::Relations;

                    /// Generate interaction trace for LogUp.
                    ///
                    /// Creates LogUp fractions: multiplicity / (value - z)
                    /// where `value` comes from the preprocessed column.
                    pub fn gen_interaction_trace(
                        _trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                        _relations: &Relations,
                    ) -> (
                        ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                        QM31,
                    ) {
                        // TODO: Implement LogUp interaction trace
                        // For now, return empty (scaffolding)
                        (vec![], QM31::zero())
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

                $(
                    if !traces.$table.is_empty() {
                        let log_size = traces.$table.first()
                            .map(|t| t.domain.log_size())
                            .unwrap_or(0);
                        if log_size > 0 {
                            let (interaction_trace, claimed_sum) =
                                $table::witness::gen_interaction_trace(&traces.$table, relations);
                            let trace_tree = TreeVec::new(vec![
                                vec![], // preprocessed
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
