//! `define_air_fns!`: the felt-to-AIR compiler front end
//! (docs/felt-air-compiler.md).
//!
//! Functions written as straight-line felt code compile to AIR components:
//! each function is a table, each activation a row. The calling convention
//! is a LogUp relation per function over the tuple `(inputs..., outputs...)`
//! — a row consumes its own activation tuple and emits one for every call
//! it makes; the public side emits the entry activations. The maximum
//! constraint degree is a compile parameter: multiplicative chains that
//! would breach it are unrolled into materialized intermediate columns
//! (with an equality constraint each), while additive chains stay inline.
//!
//! The lowering targets the `define_trace_tables!` backend — tables,
//! generic column structs, and the exported lookup macros — so AIR
//! evaluation, interaction-trace generation, and multiplicity bookkeeping
//! are shared with the rest of the system. The witness side is the same
//! program run concretely: the generated `call_<fn>` executes the body over
//! `BaseField` values, recursively activates callees, and pushes the rows.

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, Token, braced, parenthesized, parse_macro_input};

use crate::trace_tables::{
    DerivedDef, LookupEntry, LookupsDef, OpcodeDef, column_struct_name, const_eval,
    generate_lookup_macros, generate_prover_columns, generate_table, table_name,
};

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
// Parsing
// =============================================================================

/// One statement of a function body.
enum FnStmt {
    /// `let x = expr;` — a felt expression over the frame.
    Let { name: Ident, expr: Expr },
    /// `let (a, b) = callee(args...);` — an activation of another function.
    Call {
        rets: Vec<Ident>,
        callee: Ident,
        args: Vec<Expr>,
    },
    /// `assert lhs == rhs;`
    Assert { lhs: Expr, rhs: Expr },
}

struct AirFn {
    name: Ident,
    args: Vec<Ident>,
    body: Vec<FnStmt>,
    rets: Vec<Expr>,
}

struct AirFnsInput {
    max_degree: usize,
    fns: Vec<AirFn>,
}

impl Parse for AirFnsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "max_degree" {
            return Err(syn::Error::new(key.span(), "expected `max_degree: N,`"));
        }
        input.parse::<Token![:]>()?;
        let lit: syn::LitInt = input.parse()?;
        let max_degree: usize = lit.base10_parse()?;
        if !(2..=3).contains(&max_degree) {
            // The component shells use max_constraint_log_degree_bound =
            // log_size + 1, which admits constraint degree 3.
            return Err(syn::Error::new(
                lit.span(),
                "max_degree must be 2 or 3 (the component degree bound admits 3)",
            ));
        }
        input.parse::<Token![,]>()?;

        let mut fns = Vec::new();
        while !input.is_empty() {
            fns.push(parse_fn(input)?);
        }
        Ok(AirFnsInput { max_degree, fns })
    }
}

fn parse_fn(input: ParseStream) -> syn::Result<AirFn> {
    input.parse::<Token![fn]>()?;
    let name: Ident = input.parse()?;
    let args_content;
    parenthesized!(args_content in input);
    let args: Punctuated<Ident, Token![,]> =
        args_content.parse_terminated(Ident::parse, Token![,])?;

    let body_content;
    braced!(body_content in input);
    let mut body = Vec::new();
    let mut rets = None;
    while !body_content.is_empty() {
        if body_content.peek(Token![let]) {
            body_content.parse::<Token![let]>()?;
            // `let (a, b) = ...` or `let a = ...`
            let names: Vec<Ident> = if body_content.peek(syn::token::Paren) {
                let tuple;
                parenthesized!(tuple in body_content);
                let names: Punctuated<Ident, Token![,]> =
                    tuple.parse_terminated(Ident::parse, Token![,])?;
                names.into_iter().collect()
            } else {
                vec![body_content.parse()?]
            };
            body_content.parse::<Token![=]>()?;
            let value: Expr = body_content.parse()?;
            body_content.parse::<Token![;]>()?;
            // A direct call of a sibling function is an activation; anything
            // else is a felt expression (single binding only).
            if let Expr::Call(call) = &value
                && let Expr::Path(path) = call.func.as_ref()
                && let Some(callee) = path.path.get_ident()
                && !matches!(callee.to_string().as_str(), "pow2" | "inv" | "constant")
            {
                body.push(FnStmt::Call {
                    rets: names,
                    callee: callee.clone(),
                    args: call.args.iter().cloned().collect(),
                });
                continue;
            }
            if names.len() != 1 {
                return Err(syn::Error::new_spanned(
                    value,
                    "tuple bindings are only for function calls",
                ));
            }
            body.push(FnStmt::Let {
                name: names.into_iter().next().expect("one name"),
                expr: value,
            });
        } else if body_content.peek(Ident)
            && body_content
                .cursor()
                .ident()
                .is_some_and(|(i, _)| i == "assert")
        {
            body_content.parse::<Ident>()?;
            // `lhs == rhs` parses as one equality expression.
            let comparison: Expr = body_content.parse()?;
            body_content.parse::<Token![;]>()?;
            let Expr::Binary(binary) = comparison else {
                return Err(syn::Error::new_spanned(
                    comparison,
                    "assert takes `lhs == rhs`",
                ));
            };
            if !matches!(binary.op, syn::BinOp::Eq(_)) {
                return Err(syn::Error::new_spanned(binary, "assert takes `lhs == rhs`"));
            }
            body.push(FnStmt::Assert {
                lhs: *binary.left,
                rhs: *binary.right,
            });
        } else if body_content.peek(Token![return]) {
            body_content.parse::<Token![return]>()?;
            let exprs: Vec<Expr> = if body_content.peek(syn::token::Paren) {
                let tuple;
                parenthesized!(tuple in body_content);
                let exprs: Punctuated<Expr, Token![,]> =
                    tuple.parse_terminated(Expr::parse, Token![,])?;
                exprs.into_iter().collect()
            } else {
                vec![body_content.parse()?]
            };
            body_content.parse::<Token![;]>()?;
            if !body_content.is_empty() {
                return Err(syn::Error::new(
                    body_content.span(),
                    "return must be the last statement",
                ));
            }
            rets = Some(exprs);
        } else {
            return Err(syn::Error::new(
                body_content.span(),
                "expected `let`, `assert`, or `return`",
            ));
        }
    }
    let rets =
        rets.ok_or_else(|| syn::Error::new(name.span(), "function body must end with `return`"))?;
    Ok(AirFn {
        name,
        args: args.into_iter().collect(),
        body,
        rets,
    })
}

// =============================================================================
// Degree-budget lowering
// =============================================================================

/// What a name in scope is, with its inlined degree.
#[derive(Clone, Copy)]
enum Binding {
    /// A committed trace column (degree 1).
    Column,
    /// An inline (derived) expression of the given degree.
    Derived(usize),
}

impl Binding {
    fn degree(self) -> usize {
        match self {
            Binding::Column => 1,
            Binding::Derived(degree) => degree,
        }
    }
}

struct Lowerer {
    scope: HashMap<String, Binding>,
    /// Frame cells in creation order, for the witness fill.
    fill: Vec<FillStep>,
    /// Committed columns beyond the arguments, in creation order.
    extra_columns: Vec<Ident>,
    /// Inline (derived) expressions.
    derived: Vec<(Ident, Expr)>,
    /// Constraints: materialization equalities and asserts.
    constraints: Vec<Expr>,
    temp_counter: usize,
}

enum FillStep {
    /// `let name = expr;` over BaseField values (derived or materialized).
    Expr { name: Ident, expr: Expr },
    /// `let [rets...] = call_callee(tables, [args...]);`
    Call {
        rets: Vec<Ident>,
        callee: Ident,
        args: Vec<Expr>,
    },
}

impl Lowerer {
    fn new(args: &[Ident]) -> Self {
        let mut scope = HashMap::new();
        for arg in args {
            scope.insert(arg.to_string(), Binding::Column);
        }
        Self {
            scope,
            fill: Vec::new(),
            extra_columns: Vec::new(),
            derived: Vec::new(),
            constraints: Vec::new(),
            temp_counter: 0,
        }
    }

    /// Materialize an expression as a committed column: the cell gets an
    /// equality constraint and a fill step; its degree drops to 1.
    fn materialize(&mut self, expr: Expr, fn_name: &Ident) -> Expr {
        let name = format_ident!("{}_t{}", fn_name, self.temp_counter);
        self.temp_counter += 1;
        self.extra_columns.push(name.clone());
        self.scope.insert(name.to_string(), Binding::Column);
        self.constraints.push(syn::parse_quote!(#name - (#expr)));
        self.fill.push(FillStep::Expr {
            name: name.clone(),
            expr: expr.clone(),
        });
        syn::parse_quote!(#name)
    }

    /// Lower an expression so its degree fits `budget`, materializing
    /// multiplicative subtrees as needed. Returns the (possibly rewritten)
    /// expression and its degree.
    fn lower(&mut self, expr: &Expr, budget: usize, fn_name: &Ident) -> syn::Result<(Expr, usize)> {
        // Constant subtrees have degree 0 and stay verbatim (the backend
        // folds them at expansion time).
        if const_eval(expr).is_ok() {
            return Ok((expr.clone(), 0));
        }
        match expr {
            Expr::Path(path) => {
                let ident = path.path.get_ident().ok_or_else(|| {
                    syn::Error::new_spanned(path, "only plain names are usable in expressions")
                })?;
                let binding = self.scope.get(&ident.to_string()).ok_or_else(|| {
                    syn::Error::new_spanned(
                        ident,
                        format!("`{ident}` is not defined in this frame"),
                    )
                })?;
                Ok((expr.clone(), binding.degree()))
            }
            Expr::Call(call) => {
                // Intrinsics only; sibling-function calls are statements.
                if let Expr::Path(func) = call.func.as_ref()
                    && func
                        .path
                        .get_ident()
                        .is_some_and(|i| matches!(i.to_string().as_str(), "constant"))
                {
                    return Ok((expr.clone(), 0));
                }
                Err(syn::Error::new_spanned(
                    call,
                    "function calls must be bound directly: `let r = callee(...);`",
                ))
            }
            Expr::Binary(binary) => match binary.op {
                syn::BinOp::Add(_) | syn::BinOp::Sub(_) => {
                    let (left, dl) = self.lower(&binary.left, budget, fn_name)?;
                    let (right, dr) = self.lower(&binary.right, budget, fn_name)?;
                    let op = &binary.op;
                    Ok((syn::parse_quote!((#left #op #right)), dl.max(dr)))
                }
                syn::BinOp::Mul(_) => {
                    let (mut left, mut dl) = self.lower(&binary.left, budget, fn_name)?;
                    let (mut right, mut dr) = self.lower(&binary.right, budget, fn_name)?;
                    // Unroll the product until it fits: materialize the
                    // higher-degree side (additions never trigger this).
                    while dl + dr > budget {
                        if dl >= dr && dl > 1 {
                            left = self.materialize(left, fn_name);
                            dl = 1;
                        } else if dr > 1 {
                            right = self.materialize(right, fn_name);
                            dr = 1;
                        } else {
                            return Err(syn::Error::new_spanned(
                                binary,
                                format!("cannot reduce product below degree {}", dl + dr),
                            ));
                        }
                    }
                    Ok((syn::parse_quote!((#left * #right)), dl + dr))
                }
                _ => Err(syn::Error::new_spanned(
                    binary,
                    "only +, -, * are supported",
                )),
            },
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
                let (inner, degree) = self.lower(&unary.expr, budget, fn_name)?;
                Ok((syn::parse_quote!((-#inner)), degree))
            }
            Expr::Paren(paren) => self.lower(&paren.expr, budget, fn_name),
            Expr::Group(group) => self.lower(&group.expr, budget, fn_name),
            other => Err(syn::Error::new_spanned(
                other,
                "unsupported expression in a felt function",
            )),
        }
    }
}

/// A lowered function, ready for backend generation.
struct LoweredFn {
    name: Ident,
    args: Vec<Ident>,
    rets: Vec<Expr>,
    table: OpcodeDef,
    fill: Vec<FillStep>,
}

fn lower_fn(
    function: &AirFn,
    max_degree: usize,
    arities: &HashMap<String, (usize, usize)>,
) -> syn::Result<LoweredFn> {
    // Lookup-tuple elements appear in LogUp denominators whose singleton
    // constraint multiplies by one cumsum mask: budget max_degree - 1.
    let io_budget = max_degree - 1;
    let mut lowerer = Lowerer::new(&function.args);
    let mut calls: Vec<(Ident, Vec<Expr>, Vec<Ident>)> = Vec::new();
    let mut call_ret_columns: Vec<Ident> = Vec::new();

    for stmt in &function.body {
        match stmt {
            FnStmt::Let { name, expr } => {
                let (lowered, degree) = lowerer.lower(expr, io_budget, &function.name)?;
                lowerer
                    .scope
                    .insert(name.to_string(), Binding::Derived(degree));
                lowerer.derived.push((name.clone(), lowered));
                lowerer.fill.push(FillStep::Expr {
                    name: name.clone(),
                    expr: expr.clone(),
                });
            }
            FnStmt::Call { rets, callee, args } => {
                let (n_args, n_rets) = *arities.get(&callee.to_string()).ok_or_else(|| {
                    syn::Error::new_spanned(callee, format!("unknown function `{callee}`"))
                })?;
                if args.len() != n_args || rets.len() != n_rets {
                    return Err(syn::Error::new_spanned(
                        callee,
                        format!("`{callee}` takes {n_args} arguments and returns {n_rets} values"),
                    ));
                }
                let lowered_args = args
                    .iter()
                    .map(|arg| Ok(lowerer.lower(arg, io_budget, &function.name)?.0))
                    .collect::<syn::Result<Vec<_>>>()?;
                for ret in rets {
                    lowerer.scope.insert(ret.to_string(), Binding::Column);
                    call_ret_columns.push(ret.clone());
                }
                lowerer.fill.push(FillStep::Call {
                    rets: rets.clone(),
                    callee: callee.clone(),
                    args: args.clone(),
                });
                calls.push((callee.clone(), lowered_args, rets.clone()));
            }
            FnStmt::Assert { lhs, rhs } => {
                let (lowered, _) = lowerer.lower(
                    &syn::parse_quote!((#lhs) - (#rhs)),
                    max_degree,
                    &function.name,
                )?;
                lowerer.constraints.push(lowered);
            }
        }
    }

    let rets = function
        .rets
        .iter()
        .map(|ret| Ok(lowerer.lower(ret, io_budget, &function.name)?.0))
        .collect::<syn::Result<Vec<_>>>()?;

    // Assemble the backend table: arguments, then call returns, then
    // materialized intermediates (enabler is prepended by the backend).
    let mut fields = function.args.clone();
    fields.extend(call_ret_columns);
    fields.extend(lowerer.extra_columns.clone());

    let derived = lowerer
        .derived
        .iter()
        .map(|(name, expr)| {
            let params = referenced_idents(expr, &lowerer.scope);
            DerivedDef {
                name: name.clone(),
                closure: syn::parse_quote!(|#(#params),*| #expr),
            }
        })
        .collect();

    // The calling convention: consume the own activation tuple, emit one
    // per call made.
    let mut entries = Vec::new();
    let own_values: Vec<Expr> = function
        .args
        .iter()
        .map(|arg| syn::parse_quote!(#arg))
        .chain(rets.iter().cloned())
        .collect();
    entries.push(LookupEntry {
        preprocessed: false,
        relation: function.name.clone(),
        multiplicity: syn::parse_quote!(-enabler),
        values: own_values,
    });
    for (callee, args, ret_columns) in &calls {
        let values: Vec<Expr> = args
            .iter()
            .cloned()
            .chain(ret_columns.iter().map(|ret| syn::parse_quote!(#ret)))
            .collect();
        entries.push(LookupEntry {
            preprocessed: false,
            relation: callee.clone(),
            multiplicity: syn::parse_quote!(enabler),
            values,
        });
    }

    let table = OpcodeDef {
        name: function.name.clone(),
        fields,
        derived,
        constraints: lowerer.constraints,
        lookups: LookupsDef { batch: 1, entries },
        air_only: false,
    };

    Ok(LoweredFn {
        name: function.name.clone(),
        args: function.args.clone(),
        rets,
        table,
        fill: lowerer.fill,
    })
}

/// The in-scope names an expression references, in deterministic order —
/// the synthesized closure parameters of a derived column.
fn referenced_idents(expr: &Expr, scope: &HashMap<String, Binding>) -> Vec<Ident> {
    fn walk(expr: &Expr, scope: &HashMap<String, Binding>, out: &mut Vec<Ident>) {
        match expr {
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident()
                    && scope.contains_key(&ident.to_string())
                    && !out.iter().any(|seen| seen == ident)
                {
                    out.push(ident.clone());
                }
            }
            Expr::Binary(binary) => {
                walk(&binary.left, scope, out);
                walk(&binary.right, scope, out);
            }
            Expr::Unary(unary) => walk(&unary.expr, scope, out),
            Expr::Paren(paren) => walk(&paren.expr, scope, out),
            Expr::Group(group) => walk(&group.expr, scope, out),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(expr, scope, &mut out);
    out
}

// =============================================================================
// Code generation
// =============================================================================

/// Rewrite a body expression into concrete `BaseField` arithmetic for the
/// witness fill (names are local variables; constants fold).
fn concrete_expr(expr: &Expr) -> syn::Result<TokenStream2> {
    if let Ok(value) = const_eval(expr) {
        let value = value as u32;
        return Ok(quote! {
            stwo::core::fields::m31::BaseField::from_u32_unchecked(#value)
        });
    }
    match expr {
        Expr::Path(path) => Ok(quote!(#path)),
        Expr::Lit(lit) => {
            if let syn::Lit::Int(int) = &lit.lit {
                let value = int.base10_parse::<u32>()?;
                Ok(quote! {
                    stwo::core::fields::m31::BaseField::from_u32_unchecked(#value)
                })
            } else {
                Err(syn::Error::new_spanned(lit, "only integer literals"))
            }
        }
        Expr::Call(call) => {
            // constant(expr): verbatim u32 const from the invocation site.
            if let Expr::Path(func) = call.func.as_ref()
                && func.path.is_ident("constant")
                && call.args.len() == 1
            {
                let arg = &call.args[0];
                return Ok(quote! {
                    stwo::core::fields::m31::BaseField::from_u32_unchecked(#arg)
                });
            }
            Err(syn::Error::new_spanned(call, "unsupported call"))
        }
        Expr::Binary(binary) => {
            let left = concrete_expr(&binary.left)?;
            let right = concrete_expr(&binary.right)?;
            let op = &binary.op;
            Ok(quote!((#left #op #right)))
        }
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            let inner = concrete_expr(&unary.expr)?;
            Ok(quote!((-#inner)))
        }
        Expr::Paren(paren) => concrete_expr(&paren.expr),
        Expr::Group(group) => concrete_expr(&group.expr),
        other => Err(syn::Error::new_spanned(other, "unsupported expression")),
    }
}

pub fn define_air_fns(input: TokenStream) -> TokenStream {
    let AirFnsInput { max_degree, fns } = parse_macro_input!(input as AirFnsInput);

    let arities: HashMap<String, (usize, usize)> = fns
        .iter()
        .map(|f| (f.name.to_string(), (f.args.len(), f.rets.len())))
        .collect();

    let lowered: Vec<LoweredFn> = match fns
        .iter()
        .map(|f| lower_fn(f, max_degree, &arities))
        .collect::<syn::Result<Vec<_>>>()
    {
        Ok(lowered) => lowered,
        Err(error) => return error.to_compile_error().into(),
    };

    // Backend: tables, generic columns, exported lookup macros.
    let tables: Vec<_> = lowered.iter().map(|f| generate_table(&f.table)).collect();
    let prover_columns: Vec<_> = lowered
        .iter()
        .map(|f| generate_prover_columns(&f.table).unwrap_or_else(|e| e.to_compile_error()))
        .collect();
    let lookup_macros: Vec<_> = lowered
        .iter()
        .map(|f| generate_lookup_macros(&f.table, true))
        .collect();

    // Relations: one io relation per function over (inputs..., outputs...).
    let relation_defs: Vec<_> = lowered
        .iter()
        .map(|f| {
            let relation_type = format_ident!("{}IoRelation", to_pascal_case(&f.name.to_string()));
            let arity = f.args.len() + f.rets.len();
            quote! { stwo_constraint_framework::relation!(#relation_type, #arity); }
        })
        .collect();
    let relation_fields: Vec<_> = lowered
        .iter()
        .map(|f| {
            let name = &f.name;
            let relation_type = format_ident!("{}IoRelation", to_pascal_case(&f.name.to_string()));
            quote! { pub #name: #relation_type, }
        })
        .collect();
    let relation_dummy: Vec<_> = lowered
        .iter()
        .map(|f| {
            let name = &f.name;
            let relation_type = format_ident!("{}IoRelation", to_pascal_case(&f.name.to_string()));
            quote! { #name: #relation_type::dummy(), }
        })
        .collect();
    let relation_draw: Vec<_> = lowered
        .iter()
        .map(|f| {
            let name = &f.name;
            let relation_type = format_ident!("{}IoRelation", to_pascal_case(&f.name.to_string()));
            quote! { #name: #relation_type::draw(channel), }
        })
        .collect();

    // Tables holder + witness fill functions (the program run concretely).
    let table_fields: Vec<_> = lowered
        .iter()
        .map(|f| {
            let name = &f.name;
            let table_type = table_name(&f.name);
            quote! { pub #name: #table_type, }
        })
        .collect();
    let call_fns: Vec<_> = lowered
        .iter()
        .map(|f| generate_call_fn(f).unwrap_or_else(|e| e.to_compile_error()))
        .collect();

    // Component modules (air + witness) per function.
    let component_modules: Vec<_> = lowered.iter().map(generate_component_module).collect();

    // Prove/verify harness.
    let harness = generate_harness(&lowered);

    quote! {
        #(#tables)*

        pub mod prover_columns {
            #[allow(unused_imports)]
            use stwo_constraint_framework::EvalAtRow;

            #(#prover_columns)*
        }

        #(#lookup_macros)*

        #(#relation_defs)*

        /// The io relations: one per function, over (inputs..., outputs...).
        #[derive(Clone)]
        pub struct AirFnRelations {
            #(#relation_fields)*
        }

        impl AirFnRelations {
            pub fn dummy() -> Self {
                Self { #(#relation_dummy)* }
            }

            pub fn draw(channel: &mut impl stwo::core::channel::Channel) -> Self {
                Self { #(#relation_draw)* }
            }
        }

        /// One table per function; each activation is a row.
        #[derive(Default)]
        pub struct Tables {
            #(#table_fields)*
        }

        #(#call_fns)*

        #(#component_modules)*

        #harness
    }
    .into()
}

/// The witness fill: run the body over `BaseField`, recursively activate
/// callees, push the row, return the outputs.
fn generate_call_fn(function: &LoweredFn) -> syn::Result<TokenStream2> {
    let name = &function.name;
    let fn_name = format_ident!("call_{}", name);
    let args = &function.args;
    let n_args = args.len();
    let n_rets = function.rets.len();

    let mut steps: Vec<TokenStream2> = Vec::new();
    for step in &function.fill {
        match step {
            FillStep::Expr { name, expr } => {
                let value = concrete_expr(expr)?;
                steps.push(quote! { let #name = #value; });
            }
            FillStep::Call { rets, callee, args } => {
                let callee_fn = format_ident!("call_{}", callee);
                let arg_values = args
                    .iter()
                    .map(concrete_expr)
                    .collect::<syn::Result<Vec<_>>>()?;
                steps.push(quote! {
                    let [#(#rets),*] = #callee_fn(tables, [#(#arg_values),*]);
                });
            }
        }
    }

    // Row layout: enabler, args, call returns, materialized intermediates —
    // exactly the table's column order. Derived cells are not committed.
    let mut row_values: Vec<TokenStream2> = vec![quote!(1u32)];
    for field in &function.table.fields {
        row_values.push(quote!(#field.0));
    }
    let ret_values = function
        .rets
        .iter()
        .map(concrete_expr)
        .collect::<syn::Result<Vec<_>>>()?;

    let doc =
        format!("Activate `{name}`: run the body, recursively activate callees, push the row.");
    Ok(quote! {
        #[doc = #doc]
        pub fn #fn_name(
            tables: &mut Tables,
            args: [stwo::core::fields::m31::BaseField; #n_args],
        ) -> [stwo::core::fields::m31::BaseField; #n_rets] {
            let [#(#args),*] = args;
            #(#steps)*
            tables.#name.push_row(&[#(#row_values),*]);
            [#(#ret_values),*]
        }
    })
}

/// The component module: the same `Eval` shell every DSL component uses,
/// against the generated relations and lookup macros.
fn generate_component_module(function: &LoweredFn) -> TokenStream2 {
    let name = &function.name;
    let columns_type = column_struct_name(name);
    let lookups_macro = format_ident!("{}_lookups", name);
    let interaction_macro = format_ident!("{}_interaction", name);
    let doc = format!("{name} component, generated from its felt function.");
    quote! {
        #[doc = #doc]
        pub mod #name {
            pub mod air {
                use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

                use super::super::AirFnRelations;
                use super::super::prover_columns::#columns_type;

                pub type Component = FrameworkComponent<Eval>;

                #[derive(Clone)]
                pub struct Eval {
                    pub log_size: u32,
                    pub relations: AirFnRelations,
                }

                impl FrameworkEval for Eval {
                    fn log_size(&self) -> u32 {
                        self.log_size
                    }

                    fn max_constraint_log_degree_bound(&self) -> u32 {
                        self.log_size + 1
                    }

                    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
                        let cols = #columns_type::from_eval(&mut eval);
                        for constraint in cols.constraints() {
                            eval.add_constraint(constraint);
                        }
                        #lookups_macro!(eval, cols, self.relations);
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
                use stwo::prover::poly::BitReversedOrder;
                use stwo::prover::poly::circle::CircleEvaluation;

                use super::super::AirFnRelations;
                #[allow(unused_imports)]
                use super::super::prover_columns::#columns_type;

                pub fn gen_interaction_trace(
                    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
                    relations: &AirFnRelations,
                ) -> (
                    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                    QM31,
                ) {
                    if trace.is_empty() {
                        return (vec![], QM31::zero());
                    }
                    #interaction_macro!(trace, relations)
                }
            }
        }
    }
}

/// The prove/verify harness over all function components: single main tree,
/// relations drawn after commit, interaction traces, public activation
/// terms, stwo prove/verify.
fn generate_harness(lowered: &[LoweredFn]) -> TokenStream2 {
    let n_fns = lowered.len();
    let names: Vec<_> = lowered.iter().map(|f| f.name.clone()).collect();
    let indices: Vec<usize> = (0..n_fns).collect();

    let activation_variants: Vec<_> = lowered
        .iter()
        .map(|f| {
            let variant = format_ident!("{}", to_pascal_case(&f.name.to_string()));
            let n_args = f.args.len();
            let n_rets = f.rets.len();
            quote! {
                #variant {
                    inputs: [stwo::core::fields::m31::BaseField; #n_args],
                    outputs: [stwo::core::fields::m31::BaseField; #n_rets],
                },
            }
        })
        .collect();
    let activation_term_arms: Vec<_> = lowered
        .iter()
        .map(|f| {
            let name = &f.name;
            let variant = format_ident!("{}", to_pascal_case(&f.name.to_string()));
            quote! {
                Activation::#variant { inputs, outputs } => {
                    let tuple: Vec<stwo::core::fields::m31::BaseField> =
                        inputs.iter().chain(outputs.iter()).copied().collect();
                    let denom: stwo::core::fields::qm31::SecureField =
                        stwo_constraint_framework::Relation::combine(&relations.#name, &tuple);
                    total += stwo::core::fields::FieldExpOps::inverse(&denom);
                }
            }
        })
        .collect();
    let activation_mix_arms: Vec<_> = lowered
        .iter()
        .enumerate()
        .map(|(index, f)| {
            let variant = format_ident!("{}", to_pascal_case(&f.name.to_string()));
            let tag = index as u32;
            quote! {
                Activation::#variant { inputs, outputs } => {
                    channel.mix_u32s(&[#tag]);
                    channel.mix_u32s(&inputs.map(|v| v.0));
                    channel.mix_u32s(&outputs.map(|v| v.0));
                }
            }
        })
        .collect();

    let column_sizes: Vec<_> = lowered
        .iter()
        .map(|f| {
            let columns_type = column_struct_name(&f.name);
            quote! { prover_columns::#columns_type::<()>::SIZE }
        })
        .collect();
    let entry_counts: Vec<usize> = lowered
        .iter()
        .map(|f| f.table.lookups.entries.len())
        .collect();

    quote! {
        /// A public activation: the io tuple of an entry call the host
        /// performed. The verifier emits these (the rows consume them), so
        /// the LogUp multiset closes over exactly the requested work.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Activation {
            #(#activation_variants)*
        }

        /// The public side of the activation multiset.
        pub fn public_activation_terms(
            activations: &[Activation],
            relations: &AirFnRelations,
        ) -> stwo::core::fields::qm31::SecureField {
            use num_traits::Zero;
            let mut total = stwo::core::fields::qm31::SecureField::zero();
            for activation in activations {
                match activation {
                    #(#activation_term_arms)*
                }
            }
            total
        }

        fn mix_activations(
            channel: &mut impl stwo::core::channel::Channel,
            activations: &[Activation],
        ) {
            channel.mix_u32s(&[activations.len() as u32]);
            for activation in activations {
                match activation {
                    #(#activation_mix_arms)*
                }
            }
        }

        /// Proof of the function system plus its public claim.
        pub struct AirFnsProof<H: stwo::core::vcs_lifted::merkle_hasher::MerkleHasherLifted> {
            pub log_sizes: [u32; #n_fns],
            pub sums: [stwo::core::fields::qm31::SecureField; #n_fns],
            pub activations: Vec<Activation>,
            pub stark_proof: stwo::core::proof::StarkProof<H>,
        }

        fn build_components(
            location_allocator: &mut stwo_constraint_framework::TraceLocationAllocator,
            log_sizes: &[u32; #n_fns],
            sums: &[stwo::core::fields::qm31::SecureField; #n_fns],
            relations: &AirFnRelations,
        ) -> Vec<Box<dyn stwo::prover::ComponentProver<stwo::prover::backend::simd::SimdBackend>>> {
            vec![
                #(
                    Box::new(#names::air::Component::new(
                        location_allocator,
                        #names::air::Eval {
                            log_size: log_sizes[#indices],
                            relations: relations.clone(),
                        },
                        sums[#indices],
                    )),
                )*
            ]
        }

        /// Prove the activations: every requested call (and transitively
        /// every internal one) is a row whose constraints hold and whose io
        /// tuple closes the multiset.
        pub fn prove_air_fns(
            tables: Tables,
            activations: Vec<Activation>,
            config: stwo::core::pcs::PcsConfig,
        ) -> AirFnsProof<
            <stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleChannel as stwo::core::channel::MerkleChannel>::H,
        > {
            use stwo::core::channel::{Channel, MerkleChannel};
            use stwo::core::poly::circle::CanonicCoset;
            use stwo::prover::poly::circle::PolyOps;

            type MC = stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleChannel;
            type B = stwo::prover::backend::simd::SimdBackend;

            let traces = [#(tables.#names.into_witness()),*];
            let log_sizes: [u32; #n_fns] = std::array::from_fn(|i| {
                traces[i]
                    .first()
                    .map(|t| t.domain.log_size())
                    .expect("padded trace is never empty")
            });
            let max_log_size = *log_sizes.iter().max().expect("at least one function");

            let twiddles = B::precompute_twiddles(
                CanonicCoset::new(max_log_size + 2 + config.fri_config.log_blowup_factor)
                    .circle_domain()
                    .half_coset,
            );
            let channel = &mut <MC as MerkleChannel>::C::default();
            let mut commitment_scheme =
                stwo::prover::pcs::CommitmentSchemeProver::<B, MC>::new(config, &twiddles);

            // Tree 0: empty preprocessed trace.
            let mut tree_builder = commitment_scheme.tree_builder();
            tree_builder.extend_evals(vec![]);
            tree_builder.commit(channel);

            channel.mix_u32s(&log_sizes);
            mix_activations(channel, &activations);

            // Tree 1: all function tables in declaration order.
            let mut tree_builder = commitment_scheme.tree_builder();
            tree_builder.extend_evals(traces.iter().flatten().cloned().collect::<Vec<_>>());
            tree_builder.commit(channel);

            let relations = AirFnRelations::draw(channel);

            let mut sums = [stwo::core::fields::qm31::SecureField::default(); #n_fns];
            let mut interaction_columns = Vec::new();
            #(
                {
                    let (columns, sum) =
                        #names::witness::gen_interaction_trace(&traces[#indices], &relations);
                    interaction_columns.extend(columns);
                    sums[#indices] = sum;
                }
            )*
            channel.mix_felts(&sums);

            // Tree 2: interaction traces.
            let mut tree_builder = commitment_scheme.tree_builder();
            tree_builder.extend_evals(interaction_columns);
            tree_builder.commit(channel);

            let mut location_allocator = stwo_constraint_framework::TraceLocationAllocator::default();
            let components = build_components(&mut location_allocator, &log_sizes, &sums, &relations);
            let component_refs: Vec<&dyn stwo::prover::ComponentProver<B>> =
                components.iter().map(|c| c.as_ref()).collect();

            let stark_proof = stwo::prover::prove(&component_refs, channel, commitment_scheme)
                .expect("air fns proof generation failed");

            AirFnsProof {
                log_sizes,
                sums,
                activations,
                stark_proof,
            }
        }

        /// Verify: the claimed sums plus the public activation terms must
        /// cancel, and the stwo proof must hold.
        pub fn verify_air_fns(
            proof: AirFnsProof<
                <stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleChannel as stwo::core::channel::MerkleChannel>::H,
            >,
            config: stwo::core::pcs::PcsConfig,
        ) -> Result<(), stwo::core::verifier::VerificationError> {
            use num_traits::Zero;
            use stwo::core::channel::{Channel, MerkleChannel};

            type MC = stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleChannel;

            let channel = &mut <MC as MerkleChannel>::C::default();
            let mut commitment_scheme =
                stwo::core::pcs::CommitmentSchemeVerifier::<MC>::new(config);

            let commitments = &proof.stark_proof.commitments;
            commitment_scheme.commit(commitments[0], &[], channel);
            channel.mix_u32s(&proof.log_sizes);
            mix_activations(channel, &proof.activations);

            let column_log_sizes: Vec<u32> = [#(#column_sizes),*]
                .iter()
                .zip(proof.log_sizes.iter())
                .flat_map(|(&width, &log_size)| std::iter::repeat_n(log_size, width))
                .collect();
            commitment_scheme.commit(commitments[1], &column_log_sizes, channel);

            let relations = AirFnRelations::draw(channel);

            // Every activation tuple consumed by a row must be emitted by a
            // caller row or publicly: the multiset closes exactly over the
            // requested activations.
            let total = proof.sums.iter().copied().sum::<stwo::core::fields::qm31::SecureField>()
                + public_activation_terms(&proof.activations, &relations);
            if !total.is_zero() {
                return Err(stwo::core::verifier::VerificationError::InvalidStructure(
                    "activation multiset does not close over the public activations".to_string(),
                ));
            }
            channel.mix_felts(&proof.sums);

            let interaction_log_sizes: Vec<u32> = [#(#entry_counts),*]
                .iter()
                .zip(proof.log_sizes.iter())
                .flat_map(|(&entries, &log_size)| std::iter::repeat_n(log_size, entries * 4))
                .collect();
            commitment_scheme.commit(commitments[2], &interaction_log_sizes, channel);

            let mut location_allocator = stwo_constraint_framework::TraceLocationAllocator::default();
            let components = build_components(
                &mut location_allocator,
                &proof.log_sizes,
                &proof.sums,
                &relations,
            );
            let verifier_refs: Vec<&dyn stwo::core::air::Component> = components
                .iter()
                .map(|c| c.as_ref() as &dyn stwo::core::air::Component)
                .collect();

            stwo::core::verifier::verify(&verifier_refs, channel, &mut commitment_scheme, proof.stark_proof)
        }
    }
}
