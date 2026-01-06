//! Proc-macros for generating AIR component infrastructure.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token};

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

// =============================================================================
// opcode_components! macro
// =============================================================================

/// Input for opcode_components: `opcode1, opcode2, ...`
struct OpcodeList {
    opcodes: Vec<Ident>,
}

impl Parse for OpcodeList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let opcodes: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(input)?;
        Ok(OpcodeList {
            opcodes: opcodes.into_iter().collect(),
        })
    }
}

pub fn opcode_components(input: TokenStream) -> TokenStream {
    let OpcodeList { opcodes } = syn::parse_macro_input!(input as OpcodeList);

    // Generate Traces struct fields
    let traces_fields = opcodes.iter().map(|op| {
        quote! {
            pub #op: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        }
    });

    // Generate Traces::max_log_size() and log_sizes() body
    let log_sizes_body = opcodes.iter().map(|op| {
        quote! {
            if let Some(first) = self.#op.first() {
                sizes.push(first.domain.log_size());
            }
        }
    });

    // Generate Traces::columns_cloned() body
    let columns_cloned_body = opcodes.iter().map(|op| {
        quote! {
            columns.extend(self.#op.clone());
        }
    });

    // Generate Traces::into_columns() body
    let into_columns_body = opcodes.iter().map(|op| {
        quote! {
            columns.extend(self.#op);
        }
    });

    // Generate Traces::print_tables() body
    let print_tables_body = opcodes.iter().map(|op| {
        let op_str = op.to_string();
        let pascal = to_pascal_case(&op_str);
        let columns_type = format_ident!("{}Columns", pascal);
        quote! {
            if !self.#op.is_empty() {
                let table_name = #op_str;
                let names = runner::trace::prover_columns::#columns_type::<()>::NAMES;
                let table = self.#op.to_table_named(names);
                println!("\n=== {} ({} rows) ===", table_name, self.#op.first().unwrap().values.to_cpu().len());
                println!("{}", table);
            }
        }
    });

    // Generate Claim struct fields and From impl
    let claim_fields = opcodes.iter().map(|op| {
        quote! { pub #op: u32, }
    });

    let claim_from_body = opcodes.iter().map(|op| {
        quote! {
            #op: traces.#op
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
        }
    });

    // Generate Claim::mix_into() body
    let claim_mix_into_body = opcodes.iter().map(|op| {
        quote! {
            channel.mix_u64(self.#op as u64);
        }
    });

    // Generate Claim::log_sizes() body
    let claim_log_sizes_body = opcodes.iter().map(|op| {
        let op_str = op.to_string();
        let pascal = to_pascal_case(&op_str);
        let columns_type = format_ident!("{}Columns", pascal);
        quote! {
            let count = runner::trace::prover_columns::#columns_type::<()>::SIZE;
            sizes.extend(std::iter::repeat(self.#op).take(count));
        }
    });

    // Generate ClaimedSum fields
    let claimed_sum_fields = opcodes.iter().map(|op| {
        quote! { pub #op: QM31, }
    });

    // Generate ClaimedSum::sum() body
    let claimed_sum_body = opcodes.iter().map(|op| {
        quote! { total += self.#op; }
    });

    // Generate ClaimedSum::mix_into() body
    let claimed_sum_mix_into = opcodes.iter().map(|op| {
        quote! {
            channel.mix_felts(&[self.#op]);
        }
    });

    // Generate Components struct fields
    let components_fields = opcodes.iter().map(|op| {
        quote! {
            pub #op: #op::air::Component,
        }
    });

    // Generate gen_trace() body
    let gen_trace_body = opcodes.iter().map(|op| {
        quote! {
            #op: tracer.#op.into_witness(),
        }
    });

    let register_multiplicities_body = opcodes.iter().map(|op| {
        quote! {
            #op::witness::register_multiplicities(traces.#op.as_slice(), counters);
        }
    });

    // Generate gen_interaction_trace() body
    let gen_interaction_trace_vars = opcodes.iter().map(|op| {
        let claimed_var = format_ident!("{}_claimed", op);
        quote! {
            let (cols, claimed) = #op::witness::gen_interaction_trace(
                traces.#op.as_slice(),
                relations,
            );
            all_columns.extend(cols);
            let #claimed_var = claimed;
        }
    });

    let claimed_sum_inits = opcodes.iter().map(|op| {
        let claimed_var = format_ident!("{}_claimed", op);
        quote! {
            #op: #claimed_var,
        }
    });

    // Generate Components::new() body
    let components_new_body = opcodes.iter().map(|op| {
        quote! {
            #op: #op::air::Component::new(
                location_allocator,
                #op::air::Eval {
                    log_size: claim.#op,
                    relations: relations.clone(),
                },
                claimed_sum.#op,
            ),
        }
    });

    // Generate Components::provers() body
    let provers_body = opcodes.iter().map(|op| {
        quote! { &self.#op, }
    });

    // Generate Components::verifiers() body
    let verifiers_body = opcodes.iter().map(|op| {
        quote! { &self.#op as &dyn stwo::core::air::Component, }
    });

    // Generate relation_entries() body
    let relation_entries_body = if opcodes.is_empty() {
        quote! { std::iter::empty() }
    } else {
        let chain_items = opcodes.iter().map(|op| {
            quote! { add_to_relation_entries(&self.#op, trace) }
        });
        quote! {
            itertools::chain!(#(#chain_items),*)
        }
    };

    // Generate trace_log_degree_bounds() body
    let trace_log_degree_bounds_body = opcodes.iter().map(|op| {
        quote! { self.#op.trace_log_degree_bounds(), }
    });

    // Generate assert_constraints_on_polys() body
    let assert_constraints_body = opcodes.iter().map(|op| {
        let op_str = op.to_string();
        quote! {
            if !traces.#op.is_empty() {
                let log_size = traces.#op.first()
                    .map(|t| t.domain.log_size())
                    .unwrap_or(0);
                if log_size > 0 {
                    let (interaction_trace, claimed_sum) =
                        #op::witness::gen_interaction_trace(traces.#op.as_slice(), relations);
                    let trace_tree = TreeVec::new(vec![
                        vec![], // preprocessed
                        traces.#op.clone(),
                        interaction_trace,
                    ]);
                    let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                    let eval = #op::air::Eval {
                        log_size,
                        relations: relations.clone(),
                    };
                    info!("Testing {} constraints (log_size={})", #op_str, log_size);
                    assert_constraints_on_polys(&trace_polys, CanonicCoset::new(log_size),
                        |assert_eval| { eval.evaluate(assert_eval); }, claimed_sum);
                    info!("{} constraints OK", #op_str);
                }
            }
        }
    });

    quote! {
        use stwo::core::fields::qm31::QM31;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::ColumnVec;
        use stwo::core::air::Component as AirComponent;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::poly::BitReversedOrder;

        /// Trace columns for all components.
        pub struct Traces {
            #(#traces_fields)*
        }

        impl Traces {
            pub fn max_log_size(&self) -> u32 {
                self.log_sizes().into_iter().max().unwrap_or(4)
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                #(#log_sizes_body)*
                sizes
            }

            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#columns_cloned_body)*
                columns
            }

            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#into_columns_body)*
                columns
            }

            pub fn print_tables(&self, max_rows: Option<usize>, max_cols: Option<usize>) {
                use debug_utils::ToTable;
                use stwo::prover::backend::Column;
                debug_utils::set_display_options(max_rows, max_cols);
                #(#print_tables_body)*
            }
        }

        #[derive(Debug, Clone)]
        pub struct Claim {
            #(#claim_fields)*
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    #(#claim_from_body)*
                }
            }
        }

        impl Claim {
            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claim_mix_into_body)*
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                #(#claim_log_sizes_body)*
                sizes
            }
        }

        #[derive(Clone, Debug)]
        pub struct ClaimedSum {
            #(#claimed_sum_fields)*
        }

        impl ClaimedSum {
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                #(#claimed_sum_body)*
                total
            }

            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claimed_sum_mix_into)*
            }
        }

        pub struct Components {
            #(#components_fields)*
        }

        pub fn gen_trace(
            tracer: runner::trace::Tracer,
            counters: &mut crate::relations::Counters,
        ) -> Traces {
            let traces = Traces {
                #(#gen_trace_body)*
            };
            #(#register_multiplicities_body)*
            traces
        }

        pub fn gen_interaction_trace(
            traces: &Traces,
            relations: &crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            #(#gen_interaction_trace_vars)*

            let claimed_sum = ClaimedSum {
                #(#claimed_sum_inits)*
            };

            (all_columns, claimed_sum)
        }

        impl Components {
            pub fn new(
                claim: &Claim,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    #(#components_new_body)*
                }
            }

            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                vec![ #(#provers_body)* ]
            }

            pub fn verifiers(&self) -> Vec<&dyn stwo::core::air::Component> {
                vec![ #(#verifiers_body)* ]
            }

            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                #relation_entries_body.collect()
            }

            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                vec![
                    #(#trace_log_degree_bounds_body)*
                ]
            }

            pub fn assert_constraints_on_polys(
                traces: &Traces,
                relations: &crate::relations::Relations,
            ) {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
                use tracing::info;

                #(#assert_constraints_body)*
            }
        }
    }
    .into()
}

// =============================================================================
// preprocessed_components! macro
// =============================================================================

pub fn preprocessed_components(input: TokenStream) -> TokenStream {
    let OpcodeList { opcodes: tables } = syn::parse_macro_input!(input as OpcodeList);

    // Generate inner modules for each preprocessed component
    let inner_modules = tables.iter().map(|table| {
        quote! {
            pub mod #table {
                //! Preprocessed multiplicity component.

                pub mod air {
                    use stwo_constraint_framework::{
                        EvalAtRow, FrameworkComponent, FrameworkEval, RelationEntry,
                    };
                    use crate::preprocessed::PreprocessedTable;
                    use crate::relations::Relations;

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
                            let multiplicity = eval.next_trace_mask();
                            let column_ids = crate::preprocessed::#table::Table::column_ids();
                            let preprocessed_cols: Vec<E::F> = column_ids
                                .iter()
                                .map(|id| eval.get_preprocessed_column(id.clone()))
                                .collect();

                            // Add to relation with negated multiplicity (emit side)
                            // Preprocessed tables emit their LogUp contributions
                            // Negation here balances the negated multiplicity stored by register_multiplicities
                            eval.add_to_relation(RelationEntry::new(
                                &self.relations.#table,
                                -E::EF::from(multiplicity),
                                &preprocessed_cols,
                            ));

                            eval.finalize_logup_in_pairs();
                            eval
                        }
                    }
                }

                pub mod witness {
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
                    use crate::preprocessed::PreprocessedTable;
                    use crate::relations::Relations;

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
                        let preprocessed_cols = crate::preprocessed::#table::Table::gen_columns();
                        let multiplicity = &trace[0].values.data;

                        // Convert multiplicity to PackedQM31 for write_col!
                        // Negate to balance the negated multiplicity stored by register_multiplicities
                        let multiplicity_qm31: Vec<PackedQM31> = multiplicity
                            .iter()
                            .map(|&m| -PackedQM31::from(m))
                            .collect();

                        let col_data: Vec<&[stwo::prover::backend::simd::m31::PackedM31]> =
                            preprocessed_cols.iter().map(|c| c.values.data.as_slice()).collect();

                        let simd_size = col_data[0].len();
                        let mut denom: Vec<PackedQM31> = Vec::with_capacity(simd_size);
                        for row in 0..simd_size {
                            let packed_m31_values: Vec<stwo::prover::backend::simd::m31::PackedM31> =
                                col_data.iter().map(|c| c[row]).collect();
                            denom.push(relations.#table.combine(&packed_m31_values));
                        }

                        // Write multiplicity / denom fraction
                        stwo_macros::write_col!(&multiplicity_qm31, &denom, logup_gen);

                        logup_gen.finalize_last()
                    }
                }
            }
        }
    });

    // Generate Traces struct fields
    let traces_fields = tables.iter().map(|table| {
        quote! {
            pub #table: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        }
    });

    // Generate Traces::from_counters() body
    let from_counters_body = tables.iter().map(|table| {
        quote! { #table: counters.#table.into_trace(), }
    });

    // Generate Traces::log_sizes() body
    let log_sizes_body = tables.iter().map(|table| {
        quote! {
            if let Some(first) = self.#table.first() {
                sizes.push(first.domain.log_size());
            }
        }
    });

    // Generate Traces::columns_cloned() body
    let columns_cloned_body = tables.iter().map(|table| {
        quote! { columns.extend(self.#table.clone()); }
    });

    // Generate Traces::into_columns() body
    let into_columns_body = tables.iter().map(|table| {
        quote! { columns.extend(self.#table); }
    });

    // Generate Traces::print_tables() body
    let print_tables_body = tables.iter().map(|table| {
        let table_str = table.to_string();
        quote! {
            if !self.#table.is_empty() {
                let table_name = #table_str;
                let column_ids = crate::preprocessed::#table::Table::column_ids();
                let names: Vec<&str> = column_ids.iter().map(|id| id.id.as_str()).collect();
                let table = self.#table.to_table_named(&names);
                println!("\n=== {} ({} rows) ===", table_name, self.#table.first().unwrap().values.to_cpu().len());
                println!("{}", table);
            }
        }
    });

    // Generate Claim fields and From impl
    let claim_fields = tables.iter().map(|table| {
        quote! { pub #table: u32, }
    });

    let claim_from_body = tables.iter().map(|table| {
        quote! {
            #table: traces.#table
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
        }
    });

    let claim_mix_into_body = tables.iter().map(|table| {
        quote! { channel.mix_u64(self.#table as u64); }
    });

    let claim_log_sizes_body = tables.iter().map(|table| {
        quote! { self.#table }
    });

    // Generate ClaimedSum fields
    let claimed_sum_fields = tables.iter().map(|table| {
        quote! { pub #table: QM31, }
    });

    let claimed_sum_body = tables.iter().map(|table| {
        quote! { total += self.#table; }
    });

    let claimed_sum_mix_into = tables.iter().map(|table| {
        quote! { channel.mix_felts(&[self.#table]); }
    });

    // Generate Components fields
    let components_fields = tables.iter().map(|table| {
        quote! { pub #table: #table::air::Component, }
    });

    // Generate gen_interaction_trace() body
    let gen_interaction_trace_vars = tables.iter().map(|table| {
        let claimed_var = format_ident!("{}_claimed", table);
        quote! {
            let (cols, claimed) = #table::witness::gen_interaction_trace(
                &traces.#table,
                relations,
            );
            all_columns.extend(cols);
            let #claimed_var = claimed;
        }
    });

    let claimed_sum_inits = tables.iter().map(|table| {
        let claimed_var = format_ident!("{}_claimed", table);
        quote! { #table: #claimed_var, }
    });

    // Generate Components::new() body
    let components_new_body = tables.iter().map(|table| {
        quote! {
            #table: #table::air::Component::new(
                location_allocator,
                #table::air::Eval {
                    log_size: claim.#table,
                    relations: relations.clone(),
                },
                claimed_sum.#table,
            ),
        }
    });

    // Generate Components::provers() body
    let provers_body = tables.iter().map(|table| {
        quote! { &self.#table, }
    });

    // Generate Components::verifiers() body
    let verifiers_body = tables.iter().map(|table| {
        quote! { &self.#table as &dyn stwo::core::air::Component, }
    });

    // Generate relation_entries() body
    let relation_entries_body = if tables.is_empty() {
        quote! { std::iter::empty() }
    } else {
        let chain_items = tables.iter().map(|table| {
            quote! { add_to_relation_entries(&self.#table, trace) }
        });
        quote! {
            itertools::chain!(#(#chain_items),*)
        }
    };

    // Generate trace_log_degree_bounds() body
    let trace_log_degree_bounds_body = tables.iter().map(|table| {
        quote! { self.#table.trace_log_degree_bounds(), }
    });

    // Generate assert_constraints_on_polys() body
    let assert_constraints_body = tables.iter().map(|table| {
        let table_str = table.to_string();
        quote! {
            if !traces.#table.is_empty() {
                let log_size = traces.#table.first()
                    .map(|t| t.domain.log_size())
                    .unwrap_or(0);
                if log_size > 0 {
                    let (interaction_trace, claimed_sum) =
                        #table::witness::gen_interaction_trace(&traces.#table, relations);

                    let preprocessed_cols = crate::preprocessed::#table::Table::gen_columns();

                    let trace_tree = TreeVec::new(vec![
                        preprocessed_cols,
                        traces.#table.clone(),
                        interaction_trace,
                    ]);
                    let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                    let eval = #table::air::Eval {
                        log_size,
                        relations: relations.clone(),
                    };
                    info!("Testing {} constraints (log_size={})", #table_str, log_size);
                    assert_constraints_on_polys(&trace_polys, CanonicCoset::new(log_size),
                        |assert_eval| { eval.evaluate(assert_eval); }, claimed_sum);
                    info!("{} constraints OK", #table_str);
                }
            }
        }
    });

    quote! {
        use stwo::core::fields::qm31::QM31;
        use stwo::core::fields::m31::BaseField;
        use stwo::core::ColumnVec;
        use stwo::core::air::Component as AirComponent;
        use stwo::prover::backend::simd::SimdBackend;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::poly::BitReversedOrder;

        #(#inner_modules)*

        pub struct Traces {
            #(#traces_fields)*
        }

        impl Traces {
            pub fn from_counters(counters: crate::relations::Counters) -> Self {
                Self {
                    #(#from_counters_body)*
                }
            }

            pub fn max_log_size(&self) -> u32 {
                self.log_sizes().into_iter().max().unwrap_or(4)
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                #(#log_sizes_body)*
                sizes
            }

            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#columns_cloned_body)*
                columns
            }

            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#into_columns_body)*
                columns
            }

            pub fn print_tables(&self, max_rows: Option<usize>, max_cols: Option<usize>) {
                use debug_utils::ToTable;
                use stwo::prover::backend::Column;
                use crate::preprocessed::PreprocessedTable;
                debug_utils::set_display_options(max_rows, max_cols);
                #(#print_tables_body)*
            }
        }

        #[derive(Debug, Clone)]
        pub struct Claim {
            #(#claim_fields)*
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    #(#claim_from_body)*
                }
            }
        }

        impl Claim {
            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claim_mix_into_body)*
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                vec![
                    #(#claim_log_sizes_body),*
                ]
            }
        }

        #[derive(Clone, Debug)]
        pub struct ClaimedSum {
            #(#claimed_sum_fields)*
        }

        impl ClaimedSum {
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                #(#claimed_sum_body)*
                total
            }

            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claimed_sum_mix_into)*
            }
        }

        pub struct Components {
            #(#components_fields)*
        }

        pub fn gen_interaction_trace(
            traces: &Traces,
            relations: &crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            #(#gen_interaction_trace_vars)*

            let claimed_sum = ClaimedSum {
                #(#claimed_sum_inits)*
            };

            (all_columns, claimed_sum)
        }

        impl Components {
            pub fn new(
                claim: &Claim,
                location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
                relations: crate::relations::Relations,
                claimed_sum: &ClaimedSum,
            ) -> Self {
                Self {
                    #(#components_new_body)*
                }
            }

            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                vec![ #(#provers_body)* ]
            }

            pub fn verifiers(&self) -> Vec<&dyn stwo::core::air::Component> {
                vec![ #(#verifiers_body)* ]
            }

            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                #relation_entries_body.collect()
            }

            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                vec![
                    #(#trace_log_degree_bounds_body)*
                ]
            }

            pub fn assert_constraints_on_polys(
                traces: &Traces,
                relations: &crate::relations::Relations,
            ) {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};
                use tracing::info;
                use crate::preprocessed::PreprocessedTable;

                #(#assert_constraints_body)*
            }
        }
    }
    .into()
}
