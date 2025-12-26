//! Utility macros for the prover.

/// Helper macro to count identifiers
#[macro_export]
macro_rules! count_idents {
    () => { 0usize };
    ($first:ident $(, $rest:ident)*) => { 1usize + $crate::count_idents!($($rest),*) };
}

/// Macro to generate the Relations struct and its implementations.
///
/// Usage:
/// ```ignore
/// relations! {
///     program_access: addr, clk, value,
///     memory_access: addr, clk, limb_0, limb_1, limb_2, limb_3,
/// }
/// ```
///
/// Generates:
/// - `Relations` struct with `LookupElements<N>` fields (N = field count)
/// - `Default` impl using `LookupElements::dummy()`
/// - `draw(channel)` method using `LookupElements::draw(channel)`
#[macro_export]
macro_rules! relations {
    (
        $(
            $name:ident: $($field:ident),+ $(,)?
        );* $(;)?
    ) => {
        #[derive(Clone)]
        pub struct Relations {
            $(
                #[doc = concat!("Relation: (", $(stringify!($field), ", ",)+ ")")]
                pub $name: stwo_constraint_framework::logup::LookupElements<
                    { $crate::count_idents!($($field),+) }
                >,
            )*
        }

        impl Relations {

            pub fn dummy() -> Self {
                Self {
                    $(
                        $name: stwo_constraint_framework::logup::LookupElements::dummy(),
                    )*
                }
            }

            pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                Self {
                    $(
                        $name: stwo_constraint_framework::logup::LookupElements::draw(channel),
                    )*
                }
            }
        }
    };
}

/// Macro to aggregate all RV32IM components.
///
/// Usage:
/// ```ignore
/// components! {
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
macro_rules! components {
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
        pub fn gen_trace(
            tracer: runner::trace::Tracer,
        ) -> Traces {
            Traces {
                $(
                    ${concat($category, _, $opcode)}: $category::$opcode::witness::gen_trace(tracer.$opcode),
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
            /// Each component gets its log_size from its corresponding trace.
            pub fn new(
                traces: &Traces,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: $crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    $(
                        ${concat($category, _, $opcode)}: {
                            // Get log_size from trace domain (or 0 if empty)
                            let log_size = traces.${concat($category, _, $opcode)}
                                .first()
                                .map(|eval| eval.domain.log_size())
                                .unwrap_or(0);

                            $category::$opcode::air::Component::new(
                                location_allocator,
                                $category::$opcode::air::Eval {
                                    log_size,
                                    relations: relations.clone(),
                                },
                                claimed_sum.${concat($category, _, $opcode)},
                            )
                        },
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
