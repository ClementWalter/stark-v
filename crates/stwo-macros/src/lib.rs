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
//! - `tracer_components!` - Generate AIR component infrastructure for trace-backed components
//! - `components!` - Compose trace-backed and preprocessed component infrastructure

use proc_macro::TokenStream;

mod components;
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
/// # Example
/// ```ignore
/// define_trace_tables! {
///     add: { clock, pc, rd, rs1, rs2 },
///     lui: { clock, pc, rd },
///     sb: { clock, pc, rs1, rs2, mem },
/// }
/// ```
#[proc_macro]
pub fn define_trace_tables(input: TokenStream) -> TokenStream {
    trace_tables::define_trace_tables(input)
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

/// Generate AIR component infrastructure for trace-backed components.
///
/// # Example
/// ```ignore
/// tracer_components!(add, nested::mul);
/// ```
///
/// Generates:
/// - `Traces` struct with one field per trace-backed component
/// - `Claim` struct with log_size for each component
/// - `ClaimedSum` struct with QM31 field per component
/// - `Components` struct with one `air::Component` field per component
#[proc_macro]
pub fn tracer_components(input: TokenStream) -> TokenStream {
    components::tracer_components(input)
}

/// Compose trace-backed and preprocessed component infrastructure.
///
/// # Example
/// ```ignore
/// components! {
///     trace: { nested::mul, memory },
///     preprocessed: { bitwise, range_check_20 },
/// }
/// ```
#[proc_macro]
pub fn components(input: TokenStream) -> TokenStream {
    components::components(input)
}
