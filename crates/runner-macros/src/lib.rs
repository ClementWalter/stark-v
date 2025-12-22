//! Proc-macros for the runner crate.
//!
//! Provides the `#[traced]` attribute macro that rewrites `trace!(...)` calls
//! to include the function name automatically.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Expr, ExprMacro, ItemFn, Macro};

/// Visitor that rewrites `trace!(field1, field2, ...)` to `trace!(fn_name: field1, field2, ...)`
struct TraceRewriter {
    fn_name: syn::Ident,
}

impl VisitMut for TraceRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // First recurse into nested expressions
        syn::visit_mut::visit_expr_mut(self, expr);

        // Check if this is a macro call to `trace!`
        if let Expr::Macro(ExprMacro {
            mac: Macro { path, tokens, .. },
            ..
        }) = expr
        {
            // Check if the macro is `trace!`
            if path.is_ident("trace") {
                // Rewrite: trace!(fields...) -> trace!(fn_name: fields...)
                let fn_name = &self.fn_name;
                let original_tokens = tokens.clone();
                *tokens = quote! { #fn_name: #original_tokens };
            }
        }
    }
}

/// Attribute macro that rewrites `trace!(...)` calls to include the function name.
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
///     trace!(rd, rs1, rs2);  // Becomes: trace!(add: rd, rs1, rs2)
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
