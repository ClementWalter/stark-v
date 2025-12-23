//! Proc-macros for the runner crate.
//!
//! Provides the `#[traced]` attribute macro that rewrites `trace_op!(...)` calls
//! to include the function name automatically.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Expr, ExprMacro, ItemFn, Macro, Stmt, StmtMacro};

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
