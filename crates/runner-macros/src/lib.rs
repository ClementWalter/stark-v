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
    matches!(name, "rd" | "rs1" | "rs2" | "mem")
}

/// Generate the table struct name from opcode name (e.g., "add" -> "AddTable")
fn table_name(opcode: &Ident) -> Ident {
    let name = opcode.to_string();
    let capitalized = name
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect::<String>();
    format_ident!("{}Table", capitalized)
}

/// Generate columnar field declarations for a single field
fn generate_field_decls(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns: addr, prev, clk_prev, next
        // Note: clk is NOT stored - it's redundant with tracer.clk at call site
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            pub #addr: simd::AlignedVec<u32>,
            pub #prev: simd::AlignedVec<u32>,
            pub #clk_prev: simd::AlignedVec<u32>,
            pub #next: simd::AlignedVec<u32>,
        }
    } else {
        // Scalar field (clk, pc)
        quote! {
            pub #field: simd::AlignedVec<u32>,
        }
    }
}

/// Generate field initialization for new()
fn generate_field_init(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clk)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            #addr: simd::AlignedVec::new(),
            #prev: simd::AlignedVec::new(),
            #clk_prev: simd::AlignedVec::new(),
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
        // Access fields expand to 4 columns (no clk)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            #addr: simd::AlignedVec::with_capacity(cap),
            #prev: simd::AlignedVec::with_capacity(cap),
            #clk_prev: simd::AlignedVec::with_capacity(cap),
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
        // Access fields expand to 4 columns (no clk - it's available from tracer.clk)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        quote! {
            self.#addr.push(#field.addr);
            self.#prev.push(#field.prev);
            self.#clk_prev.push(#field.clk_prev);
            self.#next.push(#field.next);
        }
    } else {
        quote! {
            self.#field.push(#field);
        }
    }
}

/// Generate debug field entries for a single row
fn generate_debug_field(field: &Ident) -> proc_macro2::TokenStream {
    let name = field.to_string();
    if is_access_field(&name) {
        // Access fields expand to 4 columns (no clk)
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        let field_name = &name;
        quote! {
            .field(#field_name, &format_args!(
                "Access {{ addr: {:#x}, prev: {:#x}, clk_prev: {}, next: {:#x} }}",
                self.table.#addr[i],
                self.table.#prev[i],
                self.table.#clk_prev[i],
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
/// Enabler is always the first column.
/// Access fields expand to 10 columns:
/// - addr (1 column)
/// - prev_0..prev_3 (4 limbs for u32 value)
/// - clk_prev (1 column)
/// - next_0..next_3 (4 limbs for u32 value)
fn flatten_fields(fields: &[Ident]) -> Vec<Ident> {
    let mut result = Vec::new();

    // Enabler is always the first column
    result.push(format_ident!("enabler"));

    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            // Access fields expand to 10 columns with limbed prev/next
            result.push(format_ident!("{}_addr", name));
            // prev as 4 u8 limbs (little-endian)
            for i in 0usize..4 {
                result.push(format_ident!("{}_prev_{}", name, i));
            }
            result.push(format_ident!("{}_clk_prev", name));
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

/// Generate the into_columns body that splits u32 values into limbs.
/// This handles the conversion from the trace table's u32 storage to
/// the prover's limbed representation.
/// Enabler is always the first column.
fn generate_into_columns_body(fields: &[Ident]) -> proc_macro2::TokenStream {
    let mut column_exprs = Vec::new();

    // Enabler is always the first column
    column_exprs.push(quote! { self.enabler });

    for field in fields {
        let name = field.to_string();
        if is_access_field(&name) {
            let addr = format_ident!("{}_addr", name);
            let prev = format_ident!("{}_prev", name);
            let clk_prev = format_ident!("{}_clk_prev", name);
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

            // clk_prev column
            column_exprs.push(quote! { self.#clk_prev });

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
            // Scalar field (clk, pc) - return directly
            column_exprs.push(quote! { self.#field });
        }
    }

    quote! {
        vec![
            #(#column_exprs),*
        ]
    }
}

/// Generate column struct name (e.g., "add" -> "AddColumns")
fn column_struct_name(opcode: &Ident) -> Ident {
    let name = opcode.to_string();
    let capitalized = name
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect::<String>();
    format_ident!("{}Columns", capitalized)
}

/// Generate prover column struct for AIR evaluation
fn generate_prover_columns(opcode: &OpcodeDef) -> proc_macro2::TokenStream {
    let struct_name = column_struct_name(&opcode.name);
    let flat_fields = flatten_fields(&opcode.fields);
    let field_count = flat_fields.len();

    let owned_fields: Vec<_> = flat_fields.iter().map(|f| quote! { pub #f: T }).collect();

    let from_eval_fields: Vec<_> = flat_fields
        .iter()
        .map(|f| quote! { #f: eval.next_trace_mask() })
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

            /// Construct from an AIR evaluator by reading trace masks.
            #[inline(always)]
            pub fn from_eval<E>(eval: &mut E) -> Self
            where E: EvalAtRow<F = T>
            {
                Self {
                    #(#from_eval_fields),*
                }
            }
        }
    }
}

/// Generate a single table struct and impl
fn generate_table(opcode: &OpcodeDef) -> proc_macro2::TokenStream {
    let struct_name = table_name(&opcode.name);

    let field_decls: Vec<_> = opcode.fields.iter().map(generate_field_decls).collect();
    let field_inits: Vec<_> = opcode.fields.iter().map(generate_field_init).collect();
    let field_inits_cap: Vec<_> = opcode.fields.iter().map(generate_field_init_cap).collect();
    let push_params: Vec<_> = opcode.fields.iter().map(generate_push_param).collect();
    let push_stmts: Vec<_> = opcode.fields.iter().map(generate_push_stmt).collect();
    let debug_fields: Vec<_> = opcode.fields.iter().map(generate_debug_field).collect();

    // Generate into_columns body that splits u32 values into limbs
    // (enabler is prepended separately in the body)
    let into_columns_body = generate_into_columns_body(&opcode.fields);

    quote! {
        #[derive(Clone, Default)]
        pub struct #struct_name {
            /// Enabler column: 1 for real rows, 0 for padding.
            pub enabler: simd::AlignedVec<u32>,
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
                                .field("enabler", &self.table.enabler[i])
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
                    enabler: simd::AlignedVec::new(),
                    #(#field_inits)*
                }
            }

            pub fn with_capacity(cap: usize) -> Self {
                Self {
                    enabler: simd::AlignedVec::with_capacity(cap),
                    #(#field_inits_cap)*
                }
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.enabler.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.enabler.is_empty()
            }

            #[inline]
            pub fn push(&mut self, #(#push_params),*) {
                self.enabler.push(1);
                #(#push_stmts)*
            }

            /// Consumes the table and returns columns as a Vec in canonical order.
            /// Order matches the column struct field order.
            /// Enabler is the first column, followed by other fields.
            /// Access fields have prev/next split into 4 u8 limbs (little-endian).
            pub fn into_columns(self) -> Vec<simd::AlignedVec<u32>> {
                #into_columns_body
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

    quote! {
        /// Main tracer structure holding all per-opcode columnar trace tables.
        pub struct Tracer {
            /// Global clock counter, incremented by 1 at each instruction.
            pub clk: u32,
            /// Maximum allowed clock difference between consecutive accesses.
            pub max_clock_diff: u32,

            /// Last access clock for each register (0-31).
            pub reg_clk: [u32; 32],
            /// Last access clock for each memory address.
            pub mem_clk: rustc_hash::FxHashMap<u32, u32>,

            /// Intermediate register clock update accesses (gap-filling).
            pub reg_clk_update: AccessTable,
            /// Intermediate memory clock update accesses (gap-filling).
            pub mem_clk_update: AccessTable,

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
                    .field("clk", &self.clk)
                    .field("max_clock_diff", &self.max_clock_diff)
                    .field("reg_clk", &self.reg_clk)
                    .field("mem_clk", &HexKeyMap(&self.mem_clk))
                    .field("reg_clk_update", &self.reg_clk_update)
                    .field("mem_clk_update", &self.mem_clk_update)
                    #(#debug_table_fields)*
                    .finish()
            }
        }

        impl Default for Tracer {
            fn default() -> Self {
                Self {
                    clk: 0,
                    max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
                    reg_clk: [0; 32],
                    mem_clk: rustc_hash::FxHashMap::default(),
                    reg_clk_update: AccessTable::new(),
                    mem_clk_update: AccessTable::new(),
                    #(#table_inits,)*
                }
            }
        }

        impl Tracer {
            /// Create a new tracer with custom max clock diff.
            pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
                Self {
                    max_clock_diff,
                    reg_clk_update: AccessTable::with_max_clock_diff(max_clock_diff),
                    mem_clk_update: AccessTable::with_max_clock_diff(max_clock_diff),
                    ..Default::default()
                }
            }

            /// Create a new tracer with pre-allocated capacity.
            pub fn with_capacity(est_instructions: usize) -> Self {
                let cap = est_instructions / 40 + 1;
                Self {
                    clk: 0,
                    max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
                    reg_clk: [0; 32],
                    mem_clk: rustc_hash::FxHashMap::default(),
                    reg_clk_update: AccessTable::new(),
                    mem_clk_update: AccessTable::new(),
                    #(#table_inits_cap,)*
                }
            }

            /// Total number of traced instructions.
            pub fn total_traces(&self) -> usize {
                0 #(#total_traces_sum)*
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
            // Filter out clk and pc - they're added automatically
            let user_fields: Vec<_> = op
                .fields
                .iter()
                .filter(|f| {
                    let s = f.to_string();
                    s != "clk" && s != "pc"
                })
                .collect();

            let field_patterns: Vec<_> = user_fields.iter().map(|f| quote! { $#f:expr }).collect();
            let field_args: Vec<_> = user_fields.iter().map(|f| quote! { $#f }).collect();

            quote! {
                (#name: $tracer:expr, $pc:expr, #(#field_patterns),*) => {
                    $tracer.#name.push($tracer.clk, $pc, #(#field_args),*);
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
///     add: { clk, pc, rd, rs1, rs2 },
///     lui: { clk, pc, rd },
///     sb: { clk, pc, rs1, rs2, mem },
/// }
/// ```
///
/// This generates:
/// - `AddTable`, `LuiTable`, `SbTable` structs with columnar fields
/// - `Tracer` struct with all tables
/// - `trace_op!` macro for recording traces
#[proc_macro]
pub fn define_trace_tables(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as TraceTablesDef);

    let tables: Vec<_> = def.opcodes.iter().map(generate_table).collect();
    let tracer = generate_tracer(&def.opcodes);
    let trace_op_macro = generate_trace_op_macro(&def.opcodes);

    // Generate prover columns
    let prover_columns: Vec<_> = def.opcodes.iter().map(generate_prover_columns).collect();

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
        }
    };

    output.into()
}
