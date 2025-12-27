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
///     alu::add, alu::sub, ...,
///     load::lb, load::lh, ...
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
    ($($category:ident :: $opcode:ident),* $(,)?) => {
        use stwo::core::fields::qm31::QM31;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::ColumnVec;
        use stwo::core::air::Component as AirComponent;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::poly::BitReversedOrder;

        /// Trace columns for all components.
        /// Field naming: `category_opcode` (e.g., `alu_add` for `alu::add`)
        pub struct Traces {
            $(
                pub ${concat($category, _, $opcode)}: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
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
                    if let Some(first) = self.${concat($category, _, $opcode)}.first() {
                        sizes.push(first.domain.log_size());
                    }
                )*
                sizes
            }

            /// Clone all columns into a flattened vec (for commitment).
            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.${concat($category, _, $opcode)}.clone());
                )*
                columns
            }

            /// Consume self and return all columns flattened.
            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                $(
                    columns.extend(self.${concat($category, _, $opcode)});
                )*
                columns
            }

        }

        /// Claim containing log_size for each component.
        /// Field naming: `category_opcode` (e.g., `alu_add` for `alu::add`)
        pub struct Claim {
            $(
                pub ${concat($category, _, $opcode)}: u32,
            )*
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    $(
                        ${concat($category, _, $opcode)}: traces.${concat($category, _, $opcode)}
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
                    channel.mix_u64(self.${concat($category, _, $opcode)} as u64);
                )*
            }
        }

        /// Claimed sums from interaction traces.
        /// Field naming: `category_opcode` (e.g., `alu_add` for `alu::add`)
        pub struct ClaimedSum {
            $(
                pub ${concat($category, _, $opcode)}: QM31,
            )*
        }

        impl ClaimedSum {
            /// Sum all claimed values.
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                $(
                    total += self.${concat($category, _, $opcode)};
                )*
                total
            }
        }

        /// AIR components for all opcodes.
        /// Field naming: `category_opcode` (e.g., `alu_add` for `alu::add`)
        pub struct Components {
            $(
                pub ${concat($category, _, $opcode)}: $category::$opcode::air::Component,
            )*
        }

        /// Generate all trace columns from tracer.
        /// Consumes the tracer and calls each component's witness::gen_trace.
        /// Counters are populated during trace generation for preprocessed lookups.
        pub fn gen_trace(
            tracer: runner::trace::Tracer,
            counters: &mut $crate::relations::Counters,
        ) -> Traces {
            Traces {
                $(
                    ${concat($category, _, $opcode)}: $category::$opcode::witness::gen_trace(tracer.$opcode, counters),
                )*
            }
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
                let (cols, claimed) = $category::$opcode::witness::gen_interaction_trace(
                    &traces.${concat($category, _, $opcode)},
                    relations,
                );
                all_columns.extend(cols);
                let ${concat($category, _, $opcode, _claimed)} = claimed;
            )*

            let claimed_sum = ClaimedSum {
                $(
                    ${concat($category, _, $opcode)}: ${concat($category, _, $opcode, _claimed)},
                )*
            };

            (all_columns, claimed_sum)
        }

        impl Components {
            /// Create all AIR components.
            /// Each component gets its log_size from the claim.
            pub fn new(
                claim: &Claim,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: $crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    $(
                        ${concat($category, _, $opcode)}: $category::$opcode::air::Component::new(
                            location_allocator,
                            $category::$opcode::air::Eval {
                                log_size: claim.${concat($category, _, $opcode)},
                                relations: relations.clone(),
                            },
                            claimed_sum.${concat($category, _, $opcode)},
                        ),
                    )*
                }
            }

            /// Get all components as trait objects for proving.
            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                vec![ $(&self.${concat($category, _, $opcode)},)* ]
            }

            /// Collect relation tracker entries from all components.
            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                itertools::chain!(
                    $( add_to_relation_entries(&self.${concat($category, _, $opcode)}, trace) ),*
                )
                .collect()
            }

            /// Collect trace log degree bounds from all components.
            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                vec![
                    $( self.${concat($category, _, $opcode)}.trace_log_degree_bounds(), )*
                ]
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
///     // xor_8,
/// }
/// ```
///
/// Generates:
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
        }
    };
}
