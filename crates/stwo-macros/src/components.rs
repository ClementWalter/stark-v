//! Proc-macros for generating AIR component infrastructure.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Path, Token};

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

struct ComponentEntry {
    name: Ident,
    module: Path,
}

impl Parse for ComponentEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first: Path = input.parse()?;
        let name = path_name(&first)?;
        let module = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            input.parse()?
        } else {
            first
        };
        Ok(Self { name, module })
    }
}

fn path_name(path: &Path) -> syn::Result<Ident> {
    path.segments
        .last()
        .map(|segment| segment.ident.clone())
        .ok_or_else(|| syn::Error::new_spanned(path, "component path must have a final segment"))
}

/// Input for opcode_components:
/// - `opcode1, opcode2, ...`
/// - `preprocessed; nested::opcode1, ...`
struct OpcodeList {
    preprocessed: Option<Path>,
    opcodes: Vec<ComponentEntry>,
}

struct IdentList {
    idents: Vec<Ident>,
}

impl Parse for IdentList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(input)?;
        Ok(Self {
            idents: idents.into_iter().collect(),
        })
    }
}

impl Parse for OpcodeList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let preprocessed = if input.peek(Ident) {
            let fork = input.fork();
            let path: Path = fork.parse()?;
            let name = path_name(&path)?;
            if name == "preprocessed" && fork.peek(Token![:]) {
                input.parse::<Path>()?;
                input.parse::<Token![:]>()?;
                let path = input.parse()?;
                input.parse::<Token![;]>()?;
                Some(path)
            } else if name == "preprocessed" && fork.peek(Token![;]) {
                let path = input.parse()?;
                input.parse::<Token![;]>()?;
                Some(path)
            } else {
                None
            }
        } else {
            None
        };

        let opcodes: Punctuated<ComponentEntry, Token![,]> = Punctuated::parse_terminated(input)?;
        let mut opcodes: Vec<ComponentEntry> = opcodes.into_iter().collect();
        let preprocessed = if preprocessed.is_none()
            && opcodes
                .first()
                .is_some_and(|component| component.name == "preprocessed")
        {
            Some(opcodes.remove(0).module)
        } else {
            preprocessed
        };
        Ok(OpcodeList {
            preprocessed,
            opcodes,
        })
    }
}

pub fn opcode_components(input: TokenStream) -> TokenStream {
    let OpcodeList {
        preprocessed,
        opcodes,
    } = syn::parse_macro_input!(input as OpcodeList);

    // Generate Traces struct fields
    let traces_fields = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            pub #op: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        }
    });
    let preprocessed_traces_field = preprocessed.as_ref().map(|path| {
        quote! {
            pub preprocessed: #path::Traces,
        }
    });

    // Generate Traces::max_log_size() and log_sizes() body
    let log_sizes_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            if let Some(first) = self.#op.first() {
                sizes.push(first.domain.log_size());
            }
        }
    });
    let preprocessed_log_sizes = preprocessed.as_ref().map(|_| {
        quote! {
            sizes.extend(self.preprocessed.log_sizes());
        }
    });

    // Generate Traces::columns_cloned() body
    let columns_cloned_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            columns.extend(self.#op.clone());
        }
    });
    let preprocessed_columns_cloned = preprocessed.as_ref().map(|_| {
        quote! {
            columns.extend(self.preprocessed.columns_cloned());
        }
    });

    // Generate Traces::into_columns() body
    let into_columns_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            columns.extend(self.#op);
        }
    });
    let preprocessed_into_columns = preprocessed.as_ref().map(|_| {
        quote! {
            columns.extend(self.preprocessed.into_columns());
        }
    });

    // Generate Traces::print_tables() body
    let print_tables_body = opcodes.iter().map(|component| {
        let op = &component.name;
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
    let preprocessed_print_tables = preprocessed.as_ref().map(|_| {
        quote! {
            self.preprocessed.print_tables(max_rows, max_cols);
        }
    });

    // Generate Claim struct fields and From impl
    let claim_fields = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { pub #op: u32, }
    });
    let preprocessed_claim_field = preprocessed.as_ref().map(|path| {
        quote! {
            pub preprocessed: #path::Claim,
        }
    });

    let claim_from_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            #op: traces.#op
                .first()
                .map(|eval| eval.domain.log_size())
                .unwrap_or(0),
        }
    });
    let preprocessed_claim_from = preprocessed.as_ref().map(|_| {
        quote! {
            preprocessed: (&traces.preprocessed).into(),
        }
    });

    // Generate Claim::mix_into() body
    let claim_mix_into_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            channel.mix_u64(self.#op as u64);
        }
    });
    let preprocessed_claim_mix_into = preprocessed.as_ref().map(|_| {
        quote! {
            self.preprocessed.mix_into(channel);
        }
    });

    // Generate Claim::log_sizes() body
    let claim_log_sizes_body = opcodes.iter().map(|component| {
        let op = &component.name;
        let op_str = op.to_string();
        let pascal = to_pascal_case(&op_str);
        let columns_type = format_ident!("{}Columns", pascal);
        quote! {
            let count = runner::trace::prover_columns::#columns_type::<()>::SIZE;
            sizes.extend(std::iter::repeat(self.#op).take(count));
        }
    });
    let preprocessed_claim_log_sizes = preprocessed.as_ref().map(|_| {
        quote! {
            sizes.extend(self.preprocessed.log_sizes());
        }
    });

    // Generate ClaimedSum fields
    let claimed_sum_fields = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { pub #op: QM31, }
    });
    let preprocessed_claimed_sum_field = preprocessed.as_ref().map(|path| {
        quote! {
            pub preprocessed: #path::ClaimedSum,
        }
    });

    // Generate ClaimedSum::sum() body
    let claimed_sum_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { total += self.#op; }
    });
    let preprocessed_claimed_sum_body = preprocessed.as_ref().map(|_| {
        quote! {
            total += self.preprocessed.sum();
        }
    });

    // Generate ClaimedSum::mix_into() body
    let claimed_sum_mix_into = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! {
            channel.mix_felts(&[self.#op]);
        }
    });
    let preprocessed_claimed_sum_mix_into = preprocessed.as_ref().map(|_| {
        quote! {
            self.preprocessed.mix_into(channel);
        }
    });

    // Generate Components struct fields
    let components_fields = opcodes.iter().map(|component| {
        let op = &component.name;
        let module = &component.module;
        quote! {
            pub #op: #module::air::Component,
        }
    });
    let preprocessed_components_field = preprocessed.as_ref().map(|path| {
        quote! {
            pub preprocessed: #path::Components,
        }
    });

    // Generate gen_trace() body
    let gen_trace_locals: Vec<_> = opcodes
        .iter()
        .map(|component| {
            let op = &component.name;
            quote! {
                let #op = tracer.#op.into_witness();
            }
        })
        .collect();
    let gen_trace_fields: Vec<_> = opcodes
        .iter()
        .map(|component| {
            let op = &component.name;
            quote! { #op, }
        })
        .collect();
    let gen_trace_counters_ref = if preprocessed.is_some() {
        quote! { &mut counters }
    } else {
        quote! { counters }
    };

    let register_multiplicities_body: Vec<_> = opcodes
        .iter()
        .map(|component| {
            let op = &component.name;
            let module = &component.module;
            quote! {
                #module::witness::register_multiplicities(#op.as_slice(), #gen_trace_counters_ref);
            }
        })
        .collect();
    let preprocessed_gen_trace_local = preprocessed.as_ref().map(|path| {
        quote! {
            let preprocessed = #path::Traces::from_counters(counters);
        }
    });
    let preprocessed_gen_trace_field = preprocessed.as_ref().map(|_| {
        quote! {
            preprocessed,
        }
    });
    let gen_trace_function = if preprocessed.is_some() {
        quote! {
            pub fn gen_trace(
                tracer: runner::trace::Tracer,
            ) -> Traces {
                let mut counters = crate::relations::Counters::new();
                #(#gen_trace_locals)*
                #(#register_multiplicities_body)*
                #preprocessed_gen_trace_local

                Traces {
                    #(#gen_trace_fields)*
                    #preprocessed_gen_trace_field
                }
            }
        }
    } else {
        quote! {
            pub fn gen_trace(
                tracer: runner::trace::Tracer,
                counters: &mut crate::relations::Counters,
            ) -> Traces {
                #(#gen_trace_locals)*
                #(#register_multiplicities_body)*

                Traces {
                    #(#gen_trace_fields)*
                }
            }
        }
    };

    // Generate gen_interaction_trace() body
    let gen_interaction_trace_vars = opcodes.iter().map(|component| {
        let op = &component.name;
        let module = &component.module;
        let claimed_var = format_ident!("{}_claimed", op);
        quote! {
            let (cols, claimed) = #module::witness::gen_interaction_trace(
                &traces.#op,
                relations,
            );
            all_columns.extend(cols);
            let #claimed_var = claimed;
        }
    });
    let preprocessed_interaction_trace = preprocessed.as_ref().map(|path| {
        quote! {
            let (preprocessed_columns, preprocessed_claimed) =
                #path::gen_interaction_trace(&traces.preprocessed, relations);
            all_columns.extend(preprocessed_columns);
        }
    });

    let claimed_sum_inits = opcodes.iter().map(|component| {
        let op = &component.name;
        let claimed_var = format_ident!("{}_claimed", op);
        quote! {
            #op: #claimed_var,
        }
    });
    let preprocessed_claimed_sum_init = preprocessed.as_ref().map(|_| {
        quote! {
            preprocessed: preprocessed_claimed,
        }
    });

    // Generate Components::new() body
    let components_new_body = opcodes.iter().map(|component| {
        let op = &component.name;
        let module = &component.module;
        quote! {
            #op: #module::air::Component::new(
                location_allocator,
                #module::air::Eval {
                    log_size: claim.#op,
                    relations: relations.clone(),
                },
                claimed_sum.#op,
            ),
        }
    });
    let preprocessed_components_new = preprocessed.as_ref().map(|path| {
        quote! {
            preprocessed: #path::Components::new(
                &claim.preprocessed,
                location_allocator,
                relations.clone(),
                &claimed_sum.preprocessed,
            ),
        }
    });

    // Generate Components::provers() body
    let provers_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { &self.#op as &dyn stwo::prover::ComponentProver<SimdBackend>, }
    });
    let preprocessed_provers = preprocessed.as_ref().map(|_| {
        quote! {
            provers.extend(self.preprocessed.provers());
        }
    });

    // Generate Components::verifiers() body
    let verifiers_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { &self.#op as &dyn stwo::core::air::Component, }
    });
    let preprocessed_verifiers = preprocessed.as_ref().map(|_| {
        quote! {
            verifiers.extend(self.preprocessed.verifiers());
        }
    });

    // Generate relation_entries() body
    let relation_entries_body = if opcodes.is_empty() {
        quote! { std::iter::empty() }
    } else {
        let chain_items = opcodes.iter().map(|component| {
            let op = &component.name;
            quote! { add_to_relation_entries(&self.#op, trace) }
        });
        quote! {
            itertools::chain!(#(#chain_items),*)
        }
    };
    let preprocessed_relation_entries = preprocessed.as_ref().map(|_| {
        quote! {
            entries.extend(self.preprocessed.relation_entries(trace));
        }
    });

    // Generate trace_log_degree_bounds() body
    let trace_log_degree_bounds_body = opcodes.iter().map(|component| {
        let op = &component.name;
        quote! { self.#op.trace_log_degree_bounds(), }
    });
    let preprocessed_trace_log_degree_bounds = preprocessed.as_ref().map(|_| {
        quote! {
            bounds.extend(self.preprocessed.trace_log_degree_bounds());
        }
    });

    // Generate assert_constraints_on_polys() body
    let assert_constraints_body = opcodes.iter().map(|component| {
        let op = &component.name;
        let module = &component.module;
        let op_str = op.to_string();
        quote! {
            if !traces.#op.is_empty() {
                let log_size = traces.#op.first()
                    .map(|t| t.domain.log_size())
                    .unwrap_or(0);
                if log_size > 0 {
                    let (interaction_trace, claimed_sum) =
                        #module::witness::gen_interaction_trace(&traces.#op, relations);
                    let trace_tree = TreeVec::new(vec![
                        vec![], // preprocessed
                        traces.#op.clone(),
                        interaction_trace,
                    ]);
                    let trace_polys = trace_tree.map_cols(|c| c.interpolate());
                    let eval = #module::air::Eval {
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
    let preprocessed_assert_constraints = preprocessed.as_ref().map(|path| {
        quote! {
            #path::Components::assert_constraints_on_polys(&traces.preprocessed, relations);
        }
    });
    let track_relations_impl = preprocessed.as_ref().map(|_| {
        quote! {
            pub fn track_relations(
                &self,
                preprocessed_trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                traces: &Traces,
            ) -> stwo_constraint_framework::relation_tracker::RelationSummary {
                use stwo::core::pcs::TreeVec;
                use stwo::prover::backend::Column;

                let preprocessed_cpu: Vec<Vec<BaseField>> = preprocessed_trace
                    .iter()
                    .map(|col| col.values.to_cpu())
                    .collect();
                let main_columns = traces.columns_cloned();
                let main_cpu: Vec<Vec<BaseField>> =
                    main_columns.iter().map(|col| col.values.to_cpu()).collect();

                let cpu_trace = TreeVec::new(vec![preprocessed_cpu, main_cpu]);
                let trace_refs = TreeVec::new(
                    cpu_trace.iter().map(|tree| tree.iter().collect()).collect(),
                );

                let entries = self.relation_entries(&trace_refs);
                stwo_constraint_framework::relation_tracker::RelationSummary::summarize_relations(
                    &entries,
                )
                .cleaned()
            }
        }
    });

    quote! {
        use serde::{Serialize, Deserialize};
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
            #preprocessed_traces_field
        }

        impl Traces {
            pub fn max_log_size(&self) -> u32 {
                self.log_sizes().into_iter().max().unwrap_or(4)
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                #(#log_sizes_body)*
                #preprocessed_log_sizes
                sizes
            }

            pub fn columns_cloned(&self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#columns_cloned_body)*
                #preprocessed_columns_cloned
                columns
            }

            pub fn into_columns(self) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
                let mut columns = vec![];
                #(#into_columns_body)*
                #preprocessed_into_columns
                columns
            }

            pub fn print_tables(&self, max_rows: Option<usize>, max_cols: Option<usize>) {
                use debug_utils::ToTable;
                use stwo::prover::backend::Column;
                debug_utils::set_display_options(max_rows, max_cols);
                #(#print_tables_body)*
                #preprocessed_print_tables
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Claim {
            #(#claim_fields)*
            #preprocessed_claim_field
        }

        impl From<&Traces> for Claim {
            fn from(traces: &Traces) -> Self {
                Self {
                    #(#claim_from_body)*
                    #preprocessed_claim_from
                }
            }
        }

        impl Claim {
            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claim_mix_into_body)*
                #preprocessed_claim_mix_into
            }

            pub fn log_sizes(&self) -> Vec<u32> {
                let mut sizes = vec![];
                #(#claim_log_sizes_body)*
                #preprocessed_claim_log_sizes
                sizes
            }

            pub fn main_trace_log_sizes(&self) -> Vec<u32> {
                self.log_sizes()
            }
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct ClaimedSum {
            #(#claimed_sum_fields)*
            #preprocessed_claimed_sum_field
        }

        impl ClaimedSum {
            pub fn sum(&self) -> QM31 {
                use num_traits::Zero;
                let mut total = QM31::zero();
                #(#claimed_sum_body)*
                #preprocessed_claimed_sum_body
                total
            }

            pub fn total(&self) -> QM31 {
                self.sum()
            }

            pub fn mix_into(&self, channel: &mut impl stwo::core::channel::Channel) {
                #(#claimed_sum_mix_into)*
                #preprocessed_claimed_sum_mix_into
            }
        }

        pub struct Components {
            #(#components_fields)*
            #preprocessed_components_field
        }

        #gen_trace_function

        pub fn gen_interaction_trace(
            traces: &Traces,
            relations: &crate::relations::Relations,
        ) -> (
            ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ClaimedSum,
        ) {
            let mut all_columns = vec![];
            #(#gen_interaction_trace_vars)*
            #preprocessed_interaction_trace

            let claimed_sum = ClaimedSum {
                #(#claimed_sum_inits)*
                #preprocessed_claimed_sum_init
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
                    #preprocessed_components_new
                }
            }

            pub fn provers(&self) -> Vec<&dyn stwo::prover::ComponentProver<SimdBackend>> {
                let mut provers = vec![ #(#provers_body)* ];
                #preprocessed_provers
                provers
            }

            pub fn verifiers(&self) -> Vec<&dyn stwo::core::air::Component> {
                let mut verifiers = vec![ #(#verifiers_body)* ];
                #preprocessed_verifiers
                verifiers
            }

            pub fn relation_entries(
                &self,
                trace: &stwo::core::pcs::TreeVec<Vec<&Vec<BaseField>>>,
            ) -> Vec<stwo_constraint_framework::relation_tracker::RelationTrackerEntry> {
                use stwo_constraint_framework::relation_tracker::add_to_relation_entries;
                let mut entries: Vec<_> = #relation_entries_body.collect();
                #preprocessed_relation_entries
                entries
            }

            pub fn trace_log_degree_bounds(&self) -> Vec<stwo::core::pcs::TreeVec<ColumnVec<u32>>> {
                let mut bounds = vec![
                    #(#trace_log_degree_bounds_body)*
                ];
                #preprocessed_trace_log_degree_bounds
                bounds
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
                #preprocessed_assert_constraints
            }

            #track_relations_impl
        }
    }
    .into()
}

// =============================================================================
// preprocessed_components! macro
// =============================================================================

pub fn preprocessed_components(input: TokenStream) -> TokenStream {
    let IdentList { idents: tables } = syn::parse_macro_input!(input as IdentList);

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
        quote! { &self.#table as &dyn stwo::prover::ComponentProver<SimdBackend>, }
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
        use serde::{Serialize, Deserialize};
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

        #[derive(Debug, Clone, Serialize, Deserialize)]
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

        #[derive(Clone, Debug, Serialize, Deserialize)]
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
