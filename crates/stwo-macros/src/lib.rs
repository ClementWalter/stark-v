//! Proc-macros for the stwo prover library.
//!
//! This crate provides all macros used by runner and prover crates:
//!
//! ## Trace Table Macros
//! - `define_trace_tables!` - Generate columnar trace tables for components
//!
//! ## Helper Macros
//! - `count_idents!` - Count the number of identifiers
//!
//! ## LogUp Protocol Macros
//! - `combine!` - Combine columns into PackedQM31 via LookupElements
//! - `emit_col!` - Write 1/denom fraction (positive contribution)
//! - `consume_col!` - Write -1/denom fraction (negative contribution)
//! - `write_col!` - Write arbitrary num/denom fraction
//! - `write_pair!` - Combine two fractions into one column
//! - `emit_pair!` - Emit pair of denominators
//! - `consume_pair!` - Consume pair of denominators
//! - `add_to_relation!` - Add LogUp relation entry in AIR constraints
//!
//! ## Prover Infrastructure Macros
//! - `relations!` - Generate Relations struct and preprocessed table infrastructure
//! - `components!` - Compose trace-backed and lookup multiplicity component infrastructure

use proc_macro::TokenStream;

mod air_fns;
mod components;
mod define_air;
mod helpers;
mod logup;
mod relations;
mod trace_tables;

// =============================================================================
// Trace Table Macros (from runner-macros)
// =============================================================================

/// Generate columnar trace tables for opcodes.
///
/// This macro generates:
/// - Per-opcode table structs (e.g., `AddTable`, `LuiTable`)
/// - `Tracer` struct containing all tables
/// - `trace_op!` macro for recording traces
/// - `prover_columns` module with column structs for AIR evaluation
///
/// Each table declares committed trace columns in `committed: { ... }` and can
/// declare `derived:` columns and `constraints:` as formulas over every column
/// in scope. Each derived column becomes a generic method on the `*Columns<T>`
/// struct usable both in AIR constraints (`T = E::F`) and in witness generation
/// (`T = PackedM31` via the `at(i)` row extractor), so the expression is written
/// exactly once.
///
/// Supported in expressions:
/// - `+`, `-`, `*`, unary `-`, parentheses
/// - integer literals and constant integer sub-expressions (folded at
///   expansion time into a single field constant)
/// - `pow2(n)`: the constant 2^n
/// - `inv(c)`: the multiplicative inverse of constant `c` (e.g. `inv(pow2(8))`)
/// - `constant(expr)`: an arbitrary `u32` const expression evaluated at the
///   invocation site (e.g. `constant(crate::decode::Opcode::Addi as u32)`)
///
/// The generated `constraints()` method returns expressions that must each be
/// zero on every row: booleanity of the enabler and of every `opcode_*_flag`
/// (always included), followed by the declared constraints. Flag tables also
/// get a synthesized `enabler()` method (sum of flags).
///
/// # Example
/// ```ignore
/// define_trace_tables! {
///     add: {
///         committed: { clock, pc, rd, rs1, rs2 },
///     },
///     lui: {
///         committed: {
///             clock, pc, rd,
///             imm_0, imm_1, imm_2,
///         },
///         derived: {
///             imm: imm_0 + pow2(4) * imm_1 + pow2(12) * imm_2,
///             pc_next: pc + 4,
///         },
///         constraints: {
///             imm_2 * (1 - imm_2),
///         },
///     },
/// }
/// ```
#[proc_macro]
pub fn define_trace_tables(input: TokenStream) -> TokenStream {
    trace_tables::define_trace_tables(input)
}

/// Unified zkVM AIR schema: relations, preprocessed lookups, and trace tables.
///
/// # Example
/// ```ignore
/// define_air! {
///     relations: {
///         memory_access: addr_space, addr, clock, limb_0, limb_1, limb_2, limb_3;
///     }
///     preprocessed: {
///         range_check_20: value;
///     }
///     trace: {
///         add: {
///             committed: { clock, pc, rd, rs1, rs2 },
///         },
///     }
/// }
/// ```
#[proc_macro]
pub fn define_air(input: TokenStream) -> TokenStream {
    define_air::define_air(input)
}

/// Generate standalone component tables: same syntax and codegen as
/// `define_trace_tables!` (tables, `prover_columns`, derived columns,
/// constraints) but without the zkVM `Tracer` and `trace_op!` macro.
///
/// For AIRs that are not opcode traces, such as the recursion verifier
/// components (see docs/recursion.md).
#[proc_macro]
pub fn define_component_tables(input: TokenStream) -> TokenStream {
    trace_tables::define_component_tables(input)
}

// =============================================================================
// Helper Macros
// =============================================================================

/// Count the number of identifiers passed as arguments.
///
/// # Example
/// ```ignore
/// let n = count_idents!(a, b, c); // n = 3
/// ```
#[proc_macro]
pub fn count_idents(input: TokenStream) -> TokenStream {
    helpers::count_idents(input)
}

// =============================================================================
// LogUp Protocol Macros
// =============================================================================

/// Combine columns into PackedQM31 via LookupElements.
///
/// # Arguments
/// * `relations` - A LookupElements instance
/// * `cols` - A list of references to column data
///
/// # Returns
/// A `Vec<PackedQM31>` containing the combined values for each SIMD row.
///
/// # Example
/// ```ignore
/// let denom = combine!(relations.program_access, [&cols.pc.data, &opcode_id_col]);
/// ```
#[proc_macro]
pub fn combine(input: TokenStream) -> TokenStream {
    logup::combine(input)
}

/// Write 1/denom fraction to interaction trace (emit/positive contribution).
///
/// # Arguments
/// * `denom` - Iterator over PackedQM31 denominators
/// * `interaction_trace` - A mutable LogupTraceGenerator reference
#[proc_macro]
pub fn emit_col(input: TokenStream) -> TokenStream {
    logup::emit_col(input)
}

/// Write -1/denom fraction to interaction trace (consume/negative contribution).
///
/// # Arguments
/// * `denom` - Iterator over PackedQM31 denominators
/// * `interaction_trace` - A mutable LogupTraceGenerator reference
#[proc_macro]
pub fn consume_col(input: TokenStream) -> TokenStream {
    logup::consume_col(input)
}

/// Write arbitrary num/denom fraction to interaction trace.
///
/// # Arguments
/// * `numerator` - Slice of PackedQM31 numerators
/// * `denom` - Slice of PackedQM31 denominators
/// * `interaction_trace` - A mutable LogupTraceGenerator reference
#[proc_macro]
pub fn write_col(input: TokenStream) -> TokenStream {
    logup::write_col(input)
}

/// Combine two fractions into one column: (n0/d0 + n1/d1) = (n0*d1 + n1*d0)/(d0*d1)
///
/// # Arguments
/// * `numerator_0`, `denom_0` - First fraction
/// * `numerator_1`, `denom_1` - Second fraction
/// * `interaction_trace` - A mutable LogupTraceGenerator reference
#[proc_macro]
pub fn write_pair(input: TokenStream) -> TokenStream {
    logup::write_pair(input)
}

/// Emit a pair of denominators: write (d0+d1)/(d0*d1).
///
/// # Arguments
/// * `denom_0`, `denom_1` - The two denominators
/// * `interaction_trace` - A mutable LogupTraceGenerator reference
#[proc_macro]
pub fn emit_pair(input: TokenStream) -> TokenStream {
    logup::emit_pair(input)
}

/// Consume a pair of denominators: write -(d0+d1)/(d0*d1).
///
/// Has two variants:
/// 1. `consume_pair!(interaction_trace; col1, col2, ...)` - consume columns in pairs
/// 2. `consume_pair!(denom_0, denom_1, interaction_trace)` - consume two specific columns
#[proc_macro]
pub fn consume_pair(input: TokenStream) -> TokenStream {
    logup::consume_pair(input)
}

/// Add a LogUp relation entry in AIR constraints.
///
/// # Arguments
/// * `eval` - The evaluator implementing `EvalAtRow`
/// * `relation` - The relation (LookupElements) to add to
/// * `numerator` - The multiplier (positive for emit, negative for consume)
/// * `col...` - The columns that form the relation tuple
///
/// # Example
/// ```ignore
/// add_to_relation!(eval, self.relations.program_access, -enabler.clone(),
///     cols.pc, cols.opcode_id, cols.rd_addr);
/// ```
#[proc_macro]
pub fn add_to_relation(input: TokenStream) -> TokenStream {
    logup::add_to_relation(input)
}

// =============================================================================
// Prover Infrastructure Macros
// =============================================================================

/// Generate Relations struct and preprocessed table infrastructure.
///
/// # Example
/// ```ignore
/// relations! {
///     relations {
///         program_access: addr, clock, value;
///         memory_access: addr, clock, limb_0, limb_1, limb_2, limb_3;
///     }
///     preprocessed {
///         range_check_20: value;
///     }
/// }
/// ```
///
/// Generates:
/// - Wrapper types for each relation implementing `Relation<F, EF>` trait
/// - `Relations` struct with wrapper types for ALL relations
/// - `PreProcessedTrace` struct for constant table data
/// - `Counters` struct for multiplicity tracking
#[proc_macro]
pub fn relations(input: TokenStream) -> TokenStream {
    relations::relations(input)
}

/// Compile felt functions to AIR components (docs/felt-air-compiler.md).
///
/// Each function is a table, each activation a row; the calling convention
/// is a LogUp relation per function over `(inputs..., outputs...)`. The
/// `max_degree` parameter drives materialization: multiplicative chains
/// that would breach it are unrolled into committed intermediate columns,
/// additive chains stay inline.
///
/// # Example
/// ```ignore
/// define_air_fns! {
///     max_degree: 3,
///     fn cube(x) {
///         let x2 = x * x;
///         return x2 * x;
///     }
///     fn f(a, b) {
///         let c = cube(a);
///         return c + b;
///     }
/// }
/// ```
#[proc_macro]
pub fn define_air_fns(input: TokenStream) -> TokenStream {
    air_fns::define_air_fns(input)
}

/// Compose trace-backed and lookup multiplicity component infrastructure.
///
/// # Example
/// ```ignore
/// components! {
///     trace: { nested::mul, memory },
///     lookup: { bitwise, range_check_20 },
/// }
/// ```
#[proc_macro]
pub fn components(input: TokenStream) -> TokenStream {
    components::components(input)
}
