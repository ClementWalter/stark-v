//! Proc-macros for the runner crate.
//!
//! Provides:
//! - `#[traced]` attribute macro that rewrites `trace_op!(...)` calls
//! - `define_trace_tables!` macro for generating columnar trace tables

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit_mut::VisitMut;
use syn::{
    Expr, ExprMacro, Ident, ItemFn, Macro, Stmt, StmtMacro, Token, braced, parse_macro_input,
};

// =============================================================================
// #[traced] attribute macro
// =============================================================================

/// Visitor that rewrites `trace_op!(field1, field2, ...)` to `trace_op!(fn_name: field1, field2, ...)`
struct TraceRewriter {
    fn_name: syn::Ident,
}

impl TraceRewriter {
    /// Check if a path refers to the trace_op macro
    fn is_trace_op_path(path: &syn::Path) -> bool {
        path.is_ident("trace_op")
            || path
                .segments
                .last()
                .map(|seg| seg.ident == "trace_op")
                .unwrap_or(false)
    }

    /// Rewrite macro tokens to include function name, tracer, and cpu.pc
    fn rewrite_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let fn_name = &self.fn_name;
        let original_tokens = tokens.clone();
        // Transform: trace_op!(field1, field2, ...)
        // To: trace_op!(fn_name: tracer, cpu.pc, field1, field2, ...)
        *tokens = quote! { #fn_name: tracer, cpu.pc, #original_tokens };
    }
}

impl VisitMut for TraceRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // First recurse into nested expressions
        syn::visit_mut::visit_expr_mut(self, expr);

        // Check if this is a macro call to `trace_op!`
        if let Expr::Macro(ExprMacro {
            mac: Macro { path, tokens, .. },
            ..
        }) = expr
            && Self::is_trace_op_path(path)
        {
            self.rewrite_tokens(tokens);
        }
    }

    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        // First recurse into nested statements
        syn::visit_mut::visit_stmt_mut(self, stmt);

        // Check if this is a macro statement
        if let Stmt::Macro(StmtMacro {
            mac: Macro { path, tokens, .. },
            ..
        }) = stmt
            && Self::is_trace_op_path(path)
        {
            self.rewrite_tokens(tokens);
        }
    }
}

/// Attribute macro that rewrites `trace_op!(...)` calls to include the function name.
///
/// # Example
///
/// ```ignore
/// #[traced]
/// pub fn add(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
///     let rs1 = cpu.read_reg(inst.rs1, tracer);
///     let rs2 = cpu.read_reg(inst.rs2, tracer);
///     let rd = cpu.write_reg(inst.rd, rs1.next.wrapping_add(rs2.next), tracer);
///
///     trace_op!(rd, rs1, rs2);  // Becomes: trace_op!(add: tracer, cpu.pc, rd, rs1, rs2)
/// }
/// ```
#[proc_macro_attribute]
pub fn traced(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let fn_name = func.sig.ident.clone();

    // Rewrite all trace!(...) calls in the function body
    let mut rewriter = TraceRewriter { fn_name };
    rewriter.visit_item_fn_mut(&mut func);

    func.into_token_stream().into()
}

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
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        let clk = format_ident!("{}_clk", name);
        quote! {
            pub #addr: simd::AlignedVec<u32>,
            pub #prev: simd::AlignedVec<u32>,
            pub #clk_prev: simd::AlignedVec<u32>,
            pub #next: simd::AlignedVec<u32>,
            pub #clk: simd::AlignedVec<u32>,
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
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        let clk = format_ident!("{}_clk", name);
        quote! {
            #addr: simd::AlignedVec::new(),
            #prev: simd::AlignedVec::new(),
            #clk_prev: simd::AlignedVec::new(),
            #next: simd::AlignedVec::new(),
            #clk: simd::AlignedVec::new(),
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
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        let clk = format_ident!("{}_clk", name);
        quote! {
            #addr: simd::AlignedVec::with_capacity(cap),
            #prev: simd::AlignedVec::with_capacity(cap),
            #clk_prev: simd::AlignedVec::with_capacity(cap),
            #next: simd::AlignedVec::with_capacity(cap),
            #clk: simd::AlignedVec::with_capacity(cap),
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
        let addr = format_ident!("{}_addr", name);
        let prev = format_ident!("{}_prev", name);
        let clk_prev = format_ident!("{}_clk_prev", name);
        let next = format_ident!("{}_next", name);
        let clk = format_ident!("{}_clk", name);
        quote! {
            self.#addr.push(#field.addr);
            self.#prev.push(#field.prev);
            self.#clk_prev.push(#field.clk_prev);
            self.#next.push(#field.next);
            self.#clk.push(#field.clk);
        }
    } else {
        quote! {
            self.#field.push(#field);
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

    // Find the first scalar field for len() (should be 'clk')
    let len_field = opcode
        .fields
        .iter()
        .find(|f| !is_access_field(&f.to_string()))
        .cloned()
        .unwrap_or_else(|| Ident::new("clk", Span::call_site()));

    quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #struct_name {
            #(#field_decls)*
        }

        impl #struct_name {
            pub fn new() -> Self {
                Self {
                    #(#field_inits)*
                }
            }

            pub fn with_capacity(cap: usize) -> Self {
                Self {
                    #(#field_inits_cap)*
                }
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.#len_field.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.#len_field.is_empty()
            }

            #[inline]
            pub fn push(&mut self, #(#push_params),*) {
                #(#push_stmts)*
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

    quote! {
        /// Main tracer structure holding all per-opcode columnar trace tables.
        #[derive(Debug)]
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

    let output = quote! {
        #(#tables)*
        #tracer
        #trace_op_macro
    };

    output.into()
}
