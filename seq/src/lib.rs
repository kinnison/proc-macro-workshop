extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::quote;

use syn::{parse_macro_input, Expr, Ident, LitInt, Result, Token};

use syn::parse::{Parse, ParseStream};

struct SeqMacroInner {
    ident: Ident,
    start: LitInt,
    end: LitInt,
    body: Expr,
}

impl Parse for SeqMacroInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        let _in: Token![in] = input.parse()?;
        let start: LitInt = input.parse()?;
        let _dot2: Token![..] = input.parse()?;
        let end: LitInt = input.parse()?;
        let body: Expr = input.parse()?;
        Ok(SeqMacroInner {
            ident,
            start,
            end,
            body,
        })
    }
}


#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SeqMacroInner);

    (match seq_(&input) {
        Ok(r) => r,
        Err(e) => e.to_compile_error(),
    })
    .into()
}

fn seq_(input: &SeqMacroInner) -> Result<TokenStream2> {
    Ok(quote! {})
}
