//! Helper proc-macros.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token};

/// Input for count_idents: a comma-separated list of identifiers
struct CountIdentsInput {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for CountIdentsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents = Punctuated::parse_terminated(input)?;
        Ok(CountIdentsInput { idents })
    }
}

/// Count the number of identifiers passed as arguments.
///
/// # Example
/// ```ignore
/// let n = count_idents!(a, b, c); // n = 3usize
/// ```
pub fn count_idents(input: TokenStream) -> TokenStream {
    let CountIdentsInput { idents } = syn::parse_macro_input!(input as CountIdentsInput);
    let count = idents.len();
    quote! { #count usize }.into()
}
