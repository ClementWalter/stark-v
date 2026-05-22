//! Proc-macros for the runner crate.
//!
//! Provides:
//! - `define_trace_tables!` macro for generating columnar trace tables

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, braced, parse_macro_input};

// =============================================================================
// define_trace_tables! proc-macro
// =============================================================================

/// A single opcode definition: `name: { field1, field2, ... }`
struct OpcodeDef {
    name: Ident,
    fields: Vec<Ident>,
}

impl Parse for OpcodeDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let content;
        braced!(content in input);
        let fields: Punctuated<Ident, Token![,]> =
            content.parse_terminated(Ident::parse, Token![,])?;
        Ok(OpcodeDef {
            name,
            fields: fields.into_iter().collect(),
        })
    }
}

/// All opcode definitions
struct TraceTablesDef {
    opcodes: Vec<OpcodeDef>,
}

impl Parse for TraceTablesDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let opcodes: Punctuated<OpcodeDef, Token![,]> =
            input.parse_terminated(OpcodeDef::parse, Token![,])?;
        Ok(TraceTablesDef {
            opcodes: opcodes.into_iter().collect(),
        })
    }
}

/// Check if a field name represents an Access type (needs flattening)
fn is_access_field(name: &str) -> bool {
    matches!(name, "rd" | "rs1" | "rs2" | "mem" | "dst" | "src")
}

/// Check if a field name is an opcode flag (matches pattern `opcode_*_flag`)
fn is_opcode_flag(name: &str) -> bool {
    name.starts_with("opcode_") && name.ends_with("_flag")
}

/// Count the number of opcode flags in the fields list.
/// Used to determine whether to include an enabler column.
fn count_opcode_flags(fields: &[Ident]) -> usize {
    fields
        .iter()
        .filter(|f| is_opcode_flag(&f.to_string()))
        .count()
}

/// Convert a snake_case identifier to PascalCase.
/// E.g., "base_alu_imm" -> "BaseAluImm"
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

/// Generate the table struct name from opcode name (e.g., "base_alu_imm" -> "BaseAluImmTable")
fn table_name(opcode: &Ident) -> Ident {
    let pascal = to_pascal_case(&opcode.to_string());
    format_ident!("{}Table", pascal)
}

/// Generate columnar field declarations for a single field
fn generate_field_decls(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns: addr, prev, clock_prev, next
        // Note: clock is NOT stored - it's redundant with tracer.clock at call site
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            pub #addr: simd::AlignedVec<u32>,
            pub #prev: simd::AlignedVec<u32>,
            pub #clock_prev: simd::AlignedVec<u32>,
            pub #next: simd::AlignedVec<u32>,
        }
    } else {
        // Scalar field (clock, pc)
        quote! {
            pub #field: simd::AlignedVec<u32>,
        }
    }
}

/// Generate field initialization for new()
fn generate_field_init(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clock)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            #addr: simd::AlignedVec::new(),
            #prev: simd::AlignedVec::new(),
            #clock_prev: simd::AlignedVec::new(),
            #next: simd::AlignedVec::new(),
        }
    } else {
        quote! {
            #field: simd::AlignedVec::new(),
        }
    }
}

/// Generate field initialization with capacity
fn generate_field_init_cap(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clock)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            #addr: simd::AlignedVec::with_capacity(cap),
            #prev: simd::AlignedVec::with_capacity(cap),
            #clock_prev: simd::AlignedVec::with_capacity(cap),
            #next: simd::AlignedVec::with_capacity(cap),
        }
    } else {
        quote! {
            #field: simd::AlignedVec::with_capacity(cap),
        }
    }
}

/// Generate push method parameter for a field
fn generate_push_param(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        quote! { #field: Access }
    } else {
        quote! { #field: u32 }
    }
}

/// Generate push statements for a field
fn generate_push_stmt(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clock - it's available from tracer.clock)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            self.#addr.push(#field.addr);
            self.#prev.push(#field.prev);
            self.#clock_prev.push(#field.clock_prev);
            self.#next.push(#field.next);
        }
    } else {
        quote! {
            self.#field.push(#field);
        }
    }
}

/// Generate push-row statements for a field from a flat row slice
fn generate_push_row_stmt(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns in trace order
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            self.#addr.push(row[idx]);
            idx += 1;
            self.#prev.push(row[idx]);
            idx += 1;
            self.#clock_prev.push(row[idx]);
            idx += 1;
            self.#next.push(row[idx]);
            idx += 1;
        }
    } else {
        quote! {
            self.#field.push(row[idx]);
            idx += 1;
        }
    }
}

/// Generate debug field entries for a single row
fn generate_debug_field(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clock)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clock_prev = format_ident!("{}_clock_prev", name);
        let next = format_ident!("{}_next", name);
        let field_name = &name;
        quote! {
            .field(#field_name, &format_args!(
                "Access {{ addr: {:#x}, prev: {:#x}, clock_prev: {}, next: {:#x} }}",
                self.table.#addr[i],
                self.table.#prev[i],
                self.table.#clock_prev[i],
                self.table.#next[i]
            ))
        }
    } else {
        let field_name = &name;
        quote! {
            .field(#field_name, &self.table.#field[i])
        }
    }
}

/// Flatten field identifiers for prover columns.
/// Enabler is the first column only if `include_enabler` is true.
/// Access fields expand to 10 columns:
/// - addr (1 column)
/// - prev_0..prev_3 (4 limbs for u32 value)
/// - clock_prev (1 column)
/// - next_0..next_3 (4 limbs for u32 value)
fn flatten_fields(fields: &[Ident], include_enabler: bool) -> Vec<Ident> {
    let mut result = Vec::new();

    // Enabler is the first column only if no opcode flags are present
    if include_enabler {
        result.push(format_ident!("enabler"));
    }

    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            // Access fields expand to 10 columns with limbed prev/next
            result.push(format_ident!("{}_addr", name));
            // prev as 4 u8 limbs (little-endian)
            for i in 0usize..4 {
                result.push(format_ident!("{}_prev_{}", name, i));
            }
            result.push(format_ident!("{}_clock_prev", name));
            // next as 4 u8 limbs (little-endian)
            for i in 0usize..4 {
                result.push(format_ident!("{}_next_{}", name, i));
            }
        } else {
            result.push(field.clone());
        }
    }
    result
}

/// Count trace columns (enabler + fields, with Access fields expanding to 4 columns).
fn trace_columns_len(fields: &[Ident], include_enabler: bool) -> usize {
    let mut count = if include_enabler { 1 } else { 0 };
    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            count += 4;
        } else {
            count += 1;
        }
    }
    count
}

/// Generate the into_columns body that splits u32 values into limbs.
/// This handles the conversion from the trace table's u32 storage to
/// the prover's limbed representation.
/// Enabler is the first column only if `include_enabler` is true.
fn generate_into_columns_body(fields: &[Ident], include_enabler: bool) -> proc_macro2::TokenStream {
    let mut column_exprs = Vec::new();

    // Enabler is the first column only if no opcode flags are present
    if include_enabler {
        column_exprs.push(quote! { self.enabler });
    }

    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            let addr = format_ident!("{}_addr", name);
            let prev = format_ident!("{}_prev", name);
            let clock_prev = format_ident!("{}_clock_prev", name);
            let next = format_ident!("{}_next", name);

            // addr column
            column_exprs.push(quote! { self.#addr });

            // prev as 4 limbs (little-endian: limb 0 is least significant byte)
            for i in 0u8..4 {
                let shift = i * 8;
                column_exprs.push(quote! {
                    {
                        let mut v = simd::AlignedVec::with_capacity(self.#prev.len());
                        for val in self.#prev.iter() {
                            v.push(((val >> #shift) & 0xFF) as u32);
                        }
                        v
                    }
                });
            }

            // clock_prev column
            column_exprs.push(quote! { self.#clock_prev });

            // next as 4 limbs (little-endian: limb 0 is least significant byte)
            for i in 0u8..4 {
                let shift = i * 8;
                column_exprs.push(quote! {
                    {
                        let mut v = simd::AlignedVec::with_capacity(self.#next.len());
                        for val in self.#next.iter() {
                            v.push(((val >> #shift) & 0xFF) as u32);
                        }
                        v
                    }
                });
            }
        } else {
            // Scalar field (clock, pc) - return directly
            column_exprs.push(quote! { self.#field });
        }
    }

    quote! {
        vec![
            #(#column_exprs),*
        ]
    }
}

/// Generate column struct name (e.g., "base_alu_imm" -> "BaseAluImmColumns")
fn column_struct_name(opcode: &Ident) -> Ident {
    let pascal = to_pascal_case(&opcode.to_string());
    format_ident!("{}Columns", pascal)
}

/// Generate Table column entries for a table (used by to_table method).
/// Returns tuples of (column_name_str, field_access_expr) for slices_to_table.
fn generate_table_columns(
    fields: &[Ident],
    include_enabler: bool,
) -> Vec<proc_macro2::TokenStream> {
    let mut columns = Vec::new();

    // Enabler first if present
    if include_enabler {
        columns.push(quote! { ("enabler", &self.enabler[..]) });
    }

    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            // Access fields have 4 columns: addr, prev, clock_prev, next
            let addr = format_ident!("{}_addr", name);
            let prev = format_ident!("{}_prev", name);
            let clock_prev = format_ident!("{}_clock_prev", name);
            let next = format_ident!("{}_next", name);

            let addr_name = format!("{name}_addr");
            let prev_name = format!("{name}_prev");
            let clock_prev_name = format!("{name}_clock_prev");
            let next_name = format!("{name}_next");

            columns.push(quote! { (#addr_name, &self.#addr[..]) });
            columns.push(quote! { (#prev_name, &self.#prev[..]) });
            columns.push(quote! { (#clock_prev_name, &self.#clock_prev[..]) });
            columns.push(quote! { (#next_name, &self.#next[..]) });
        } else {
            // Scalar field
            let field_name = name.clone();
            columns.push(quote! { (#field_name, &self.#field[..]) });
        }
    }

    columns
}

/// Generate prover column struct for AIR evaluation
fn generate_prover_columns(opcode: &OpcodeDef) -> proc_macro2::TokenStream {
    let struct_name = column_struct_name(&opcode.name);
    // Include enabler only if no opcode flags are present
    let include_enabler = count_opcode_flags(&opcode.fields) == 0;
    let flat_fields = flatten_fields(&opcode.fields, include_enabler);
    let field_count = flat_fields.len();

    let owned_fields: Vec<_> = flat_fields.iter().map(|f| quote! { pub #f: T }).collect();

    // Generate field names as strings for NAMES constant
    let field_names: Vec<String> = flat_fields.iter().map(|f| f.to_string()).collect();

    let from_eval_fields: Vec<_> = flat_fields
        .iter()
        .map(|f| quote! { #f: eval.next_trace_mask() })
        .collect();

    let from_iter_fields: Vec<_> = flat_fields
        .iter()
        .map(|f| {
            let field_name = f.to_string();
            quote! { #f: iter.next().expect(concat!("not enough columns for field: ", #field_name)) }
        })
        .collect();

    quote! {
        /// Column struct for AIR evaluation.
        #[derive(Debug, Clone)]
        pub struct #struct_name<T> {
            #(#owned_fields),*
        }

        impl<T> #struct_name<T> {
            /// Number of columns in this struct.
            pub const SIZE: usize = #field_count;

            /// Column names as strings (for debug printing).
            pub const NAMES: &'static [&'static str] = &[
                #(#field_names),*
            ];

            /// Construct from an AIR evaluator by reading trace masks.
            #[inline(always)]
            pub fn from_eval<E>(eval: &mut E) -> Self
            where E: EvalAtRow<F = T>
            {
                Self {
                    #(#from_eval_fields),*
                }
            }

            /// Construct from an iterator of column values.
            /// Panics if iterator has fewer elements than the number of columns.
            #[inline(always)]
            pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                let mut iter = iter.into_iter();
                Self {
                    #(#from_iter_fields),*
                }
            }
        }
    }
}

fn generate_clock_update_columns(name: &str) -> proc_macro2::TokenStream {
    let struct_name = format_ident!("{}ClockUpdateColumns", name);

    quote! {
        /// Column struct for clock update AIR evaluation.
        #[derive(Debug, Clone)]
        pub struct #struct_name<T> {
            pub enabler: T,
            pub addr: T,
            pub clock_prev: T,
            pub value_0: T,
            pub value_1: T,
            pub value_2: T,
            pub value_3: T,
        }

        impl<T> #struct_name<T> {
            /// Number of columns in this struct.
            pub const SIZE: usize = 7;

            /// Column names as strings.
            pub const NAMES: &'static [&'static str] = &[
                "enabler",
                "addr",
                "clock_prev",
                "value_0",
                "value_1",
                "value_2",
                "value_3",
            ];

            /// Construct from an AIR evaluator by reading trace masks.
            #[inline(always)]
            pub fn from_eval<E>(eval: &mut E) -> Self
            where E: EvalAtRow<F = T>
            {
                Self {
                    enabler: eval.next_trace_mask(),
                    addr: eval.next_trace_mask(),
                    clock_prev: eval.next_trace_mask(),
                    value_0: eval.next_trace_mask(),
                    value_1: eval.next_trace_mask(),
                    value_2: eval.next_trace_mask(),
                    value_3: eval.next_trace_mask(),
                }
            }

            /// Construct from an iterator of column values.
            /// Panics if iterator has fewer elements than the number of columns.
            #[inline(always)]
            pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                let mut iter = iter.into_iter();
                Self {
                    enabler: iter.next().expect("not enough columns for field: enabler"),
                    addr: iter.next().expect("not enough columns for field: addr"),
                    clock_prev: iter.next().expect("not enough columns for field: clock_prev"),
                    value_0: iter.next().expect("not enough columns for field: value_0"),
                    value_1: iter.next().expect("not enough columns for field: value_1"),
                    value_2: iter.next().expect("not enough columns for field: value_2"),
                    value_3: iter.next().expect("not enough columns for field: value_3"),
                }
            }
        }
    }
}

/// Generate a single table struct and impl
fn generate_table(opcode: &OpcodeDef) -> proc_macro2::TokenStream {
    let struct_name = table_name(&opcode.name);

    // Determine if we should include enabler based on opcode flags
    let include_enabler = count_opcode_flags(&opcode.fields) == 0;

    let field_decls: Vec<_> = opcode.fields.iter().map(generate_field_decls).collect();
    let field_inits: Vec<_> = opcode.fields.iter().map(generate_field_init).collect();
    let field_inits_cap: Vec<_> = opcode.fields.iter().map(generate_field_init_cap).collect();
    let push_params: Vec<_> = opcode.fields.iter().map(generate_push_param).collect();
    let push_stmts: Vec<_> = opcode.fields.iter().map(generate_push_stmt).collect();
    let push_row_stmts: Vec<_> = opcode.fields.iter().map(generate_push_row_stmt).collect();
    let debug_fields: Vec<_> = opcode.fields.iter().map(generate_debug_field).collect();

    // Generate into_columns body that splits u32 values into limbs
    let into_columns_body = generate_into_columns_body(&opcode.fields, include_enabler);
    let row_len = trace_columns_len(&opcode.fields, include_enabler);

    // Get the first field name for len/is_empty when no enabler
    // We need to find the first actual column name after expansion
    let first_field = &opcode.fields[0];
    let first_field_name = first_field.to_string();
    let len_field = if is_access_field(&first_field_name) {
        format_ident!("{}_addr", first_field_name)
    } else {
        first_field.clone()
    };

    // Conditional enabler components
    let enabler_field_decl = if include_enabler {
        quote! {
            /// Enabler column: 1 for real rows, 0 for padding.
            pub enabler: simd::AlignedVec<u32>,
        }
    } else {
        quote! {}
    };

    let enabler_field_init = if include_enabler {
        quote! { enabler: simd::AlignedVec::new(), }
    } else {
        quote! {}
    };

    let enabler_field_init_cap = if include_enabler {
        quote! { enabler: simd::AlignedVec::with_capacity(cap), }
    } else {
        quote! {}
    };

    let enabler_push_stmt = if include_enabler {
        quote! { self.enabler.push(1); }
    } else {
        quote! {}
    };

    let enabler_push_row_stmt = if include_enabler {
        quote! {
            self.enabler.push(row[idx]);
            idx += 1;
        }
    } else {
        quote! {}
    };

    let enabler_debug_field = if include_enabler {
        quote! { .field("enabler", &self.table.enabler[i]) }
    } else {
        quote! {}
    };

    let len_impl = if include_enabler {
        quote! { self.enabler.len() }
    } else {
        quote! { self.#len_field.len() }
    };

    let is_empty_impl = if include_enabler {
        quote! { self.enabler.is_empty() }
    } else {
        quote! { self.#len_field.is_empty() }
    };

    let into_columns_doc = if include_enabler {
        "Enabler is the first column, followed by other fields."
    } else {
        "No enabler column (deduced from opcode flags in AIR)."
    };

    // Generate Table column entries for to_table method
    let table_columns = generate_table_columns(&opcode.fields, include_enabler);

    quote! {
        #[derive(Clone, Default)]
        pub struct #struct_name {
            #enabler_field_decl
            #(#field_decls)*
        }

        impl std::fmt::Debug for #struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut list = f.debug_list();
                for i in 0..self.len() {
                    // Create a debug struct for each row
                    struct Row<'a> {
                        table: &'a #struct_name,
                        idx: usize,
                    }
                    impl std::fmt::Debug for Row<'_> {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            let i = self.idx;
                            f.debug_struct("")
                                #enabler_debug_field
                                #(#debug_fields)*
                                .finish()
                        }
                    }
                    list.entry(&Row { table: self, idx: i });
                }
                list.finish()
            }
        }

        impl #struct_name {
            pub fn new() -> Self {
                Self {
                    #enabler_field_init
                    #(#field_inits)*
                }
            }

            pub fn with_capacity(cap: usize) -> Self {
                Self {
                    #enabler_field_init_cap
                    #(#field_inits_cap)*
                }
            }

            #[inline]
            pub fn len(&self) -> usize {
                #len_impl
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                #is_empty_impl
            }

            #[inline]
            pub fn push(&mut self, #(#push_params),*) {
                #enabler_push_stmt
                #(#push_stmts)*
            }

            #[inline]
            pub fn push_row(&mut self, row: &[u32]) {
                debug_assert_eq!(row.len(), #row_len);
                let mut idx = 0usize;
                #enabler_push_row_stmt
                #(#push_row_stmts)*
            }

            /// Consumes the table and returns columns as a Vec in canonical order.
            /// Order matches the column struct field order.
            #[doc = #into_columns_doc]
            /// Access fields have prev/next split into 4 u8 limbs (little-endian).
            pub fn into_columns(self) -> Vec<simd::AlignedVec<u32>> {
                #into_columns_body
            }

            /// Convert table to trace columns, padding to power of 2.
            /// Always produces columns with minimum log_size of 4 (16 rows),
            /// even for empty tables.
            ///
            /// Consumes self since the table is no longer needed after trace generation.
            pub fn into_witness(
                self,
            ) -> Vec<stwo::prover::poly::circle::CircleEvaluation<
                stwo::prover::backend::simd::SimdBackend,
                stwo::core::fields::m31::BaseField,
                stwo::prover::poly::BitReversedOrder,
            >> {
                use stwo::core::poly::circle::CanonicCoset;
                use stwo::prover::backend::simd::column::BaseColumn;
                use stwo::prover::poly::circle::CircleEvaluation;

                let len = self.len() as u32;
                let log_size = len.next_power_of_two().ilog2().max(4);
                let padded_len = 1 << log_size;
                let columns = self.into_columns();
                let domain = CanonicCoset::new(log_size).circle_domain();

                columns
                    .into_iter()
                    .map(|mut col| {
                        col.resize(padded_len as usize, 0);
                        let base_col: BaseColumn = col.into();
                        CircleEvaluation::new(domain, base_col)
                    })
                    .collect()
            }

            /// Convert this table to a formatted Table for debugging.
            pub fn to_table(&self) -> debug_utils::Table {
                debug_utils::slices_to_table(&[
                    #(#table_columns),*
                ])
            }
        }
    }
}

/// Generate the Tracer struct
fn generate_tracer(opcodes: &[OpcodeDef]) -> proc_macro2::TokenStream {
    let table_fields: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            let ty = table_name(name);
            quote! { pub #name: #ty }
        })
        .collect();

    let table_inits: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            let ty = table_name(name);
            quote! { #name: #ty::new() }
        })
        .collect();

    let table_inits_cap: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            let ty = table_name(name);
            quote! { #name: #ty::with_capacity(cap) }
        })
        .collect();

    let total_traces_sum: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            quote! { + self.#name.len() }
        })
        .collect();

    let debug_table_fields: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            let name_str = name.to_string();
            quote! { .field(#name_str, &self.#name) }
        })
        .collect();

    let print_table_stmts: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            let name_str = name.to_string();
            quote! {
                if !self.#name.is_empty() {
                    println!("\n=== {} ({} rows) ===", #name_str, self.#name.len());
                    println!("{}", self.#name.to_table());
                }
            }
        })
        .collect();

    quote! {
        /// Main tracer structure holding all per-opcode columnar trace tables.
        pub struct Tracer {
            /// Global clock counter, incremented by 1 at each instruction.
            pub clock: u32,
            /// Maximum allowed clock difference between consecutive accesses.
            pub max_clock_diff: u32,

            /// Last access clock for each register (0-31).
            pub reg_clock: [u32; 32],
            /// Last access clock for each memory address.
            pub mem_clock: rustc_hash::FxHashMap<u32, u32>,
            /// Value at first access for each memory word (4-byte aligned address).
            pub mem_initial: rustc_hash::FxHashMap<u32, u32>,
            /// Program fetch counts per PC.
            pub program_reads: rustc_hash::FxHashMap<u32, u32>,

            /// Intermediate register clock update accesses (gap-filling).
            pub reg_clock_update: AccessTable,
            /// Intermediate memory clock update accesses (gap-filling).
            pub mem_clock_update: AccessTable,

            // Per-opcode trace tables
            #(#table_fields,)*
        }

        impl std::fmt::Debug for Tracer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // Wrapper to display u32 in hex
                struct Hex(u32);
                impl std::fmt::Debug for Hex {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{:#x}", self.0)
                    }
                }

                // Wrapper to display HashMap keys in hex
                struct HexKeyMap<'a>(&'a rustc_hash::FxHashMap<u32, u32>);
                impl std::fmt::Debug for HexKeyMap<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_map()
                            .entries(self.0.iter().map(|(k, v)| (Hex(*k), v)))
                            .finish()
                    }
                }

                f.debug_struct("Tracer")
                    .field("clock", &self.clock)
                    .field("max_clock_diff", &self.max_clock_diff)
                    .field("reg_clock", &self.reg_clock)
                    .field("mem_clock", &HexKeyMap(&self.mem_clock))
                    .field("mem_initial", &HexKeyMap(&self.mem_initial))
                    .field("program_reads", &HexKeyMap(&self.program_reads))
                    .field("reg_clock_update", &self.reg_clock_update)
                    .field("mem_clock_update", &self.mem_clock_update)
                    #(#debug_table_fields)*
                    .finish()
            }
        }

        impl Default for Tracer {
            fn default() -> Self {
                Self {
                    clock: 0,
                    max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
                    reg_clock: [0; 32],
                    mem_clock: rustc_hash::FxHashMap::default(),
                    mem_initial: rustc_hash::FxHashMap::default(),
                    program_reads: rustc_hash::FxHashMap::default(),
                    reg_clock_update: AccessTable::new(),
                    mem_clock_update: AccessTable::new(),
                    #(#table_inits,)*
                }
            }
        }

        impl Tracer {
            /// Create a new tracer with custom max clock diff.
            pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
                Self {
                    max_clock_diff,
                    reg_clock_update: AccessTable::with_max_clock_diff(max_clock_diff),
                    mem_clock_update: AccessTable::with_max_clock_diff(max_clock_diff),
                    ..Default::default()
                }
            }

            /// Create a new tracer with pre-allocated capacity.
            pub fn with_capacity(est_instructions: usize) -> Self {
                let cap = est_instructions / 40 + 1;
                Self {
                    clock: 0,
                    max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
                    reg_clock: [0; 32],
                    mem_clock: rustc_hash::FxHashMap::default(),
                    mem_initial: rustc_hash::FxHashMap::default(),
                    program_reads: rustc_hash::FxHashMap::default(),
                    reg_clock_update: AccessTable::new(),
                    mem_clock_update: AccessTable::new(),
                    #(#table_inits_cap,)*
                }
            }

            /// Total number of traced instructions.
            pub fn total_traces(&self) -> usize {
                0 #(#total_traces_sum)*
            }

            /// Print all non-empty trace tables as DataFrames.
            ///
            /// # Arguments
            /// * `max_rows` - Maximum rows to display per table (None for default)
            /// * `max_cols` - Maximum columns to display per table (None for default)
            pub fn print_tables(&self, max_rows: Option<usize>, max_cols: Option<usize>) {
                debug_utils::set_display_options(max_rows, max_cols);
                #(#print_table_stmts)*
            }
        }
    }
}

/// Generate the trace_op! macro
fn generate_trace_op_macro(opcodes: &[OpcodeDef]) -> proc_macro2::TokenStream {
    let arms: Vec<_> = opcodes
        .iter()
        .map(|op| {
            let name = &op.name;
            // Filter out clock and pc - they're added automatically
            let user_fields: Vec<_> = op
                .fields
                .iter()
                .filter(|f| {
                    let s = f.to_string();
                    s != "clock" && s != "pc"
                })
                .collect();

            let field_patterns: Vec<_> = user_fields.iter().map(|f| quote! { $#f:expr }).collect();
            let field_args: Vec<_> = user_fields.iter().map(|f| quote! { $#f }).collect();

            quote! {
                (#name: $tracer:expr, $pc:expr, #(#field_patterns),*) => {
                    $tracer.#name.push($tracer.clock, $pc, #(#field_args),*);
                };
            }
        })
        .collect();

    quote! {
        /// Trace macro for recording opcode execution.
        ///
        /// Usage: `trace_op!(opcode: tracer, pc, field1, field2, ...)`
        #[macro_export]
        macro_rules! trace_op {
            #(#arms)*
        }
    }
}

/// Proc-macro to define columnar trace tables.
///
/// # Example
///
/// ```ignore
/// define_trace_tables! {
///     add: { clock, pc, rd, rs1, rs2 },
///     lui: { clock, pc, rd },
///     sb: { clock, pc, rs1, rs2, mem },
/// }
/// ```
///
/// This generates:
/// - `AddTable`, `LuiTable`, `SbTable` structs with columnar fields
/// - `Tracer` struct with all tables
/// - `trace_op!` macro for recording traces
pub fn define_trace_tables(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as TraceTablesDef);

    let tables: Vec<_> = def.opcodes.iter().map(generate_table).collect();
    let tracer = generate_tracer(&def.opcodes);
    let trace_op_macro = generate_trace_op_macro(&def.opcodes);

    // Generate prover columns
    let prover_columns: Vec<_> = def.opcodes.iter().map(generate_prover_columns).collect();
    let mem_clock_update_columns = generate_clock_update_columns("Mem");
    let reg_clock_update_columns = generate_clock_update_columns("Reg");

    let output = quote! {
        // Runner code (existing)
        #(#tables)*
        #tracer
        #trace_op_macro

        // Prover columns (NEW)
        pub mod prover_columns {
            // Import EvalAtRow for from_eval method
            #[allow(unused_imports)]
            use stwo_constraint_framework::EvalAtRow;

            #(#prover_columns)*
            #mem_clock_update_columns
            #reg_clock_update_columns
        }
    };

    output.into()
}
