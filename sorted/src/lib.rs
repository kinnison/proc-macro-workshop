extern crate proc_macro;

use proc_macro::TokenStream;

use quote::ToTokens;
use syn::{parse_macro_input, Error, Item};
use proc_macro2::Span;

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let input: Item = parse_macro_input!(input as Item);

    (match sorted_impl(input) {
        Ok(t) => t.into_token_stream(),
        Err(e) => e.to_compile_error(),
    })
    .into()
}

fn sorted_impl(input: Item) -> Result<impl ToTokens, Error> {
    let ienum = match input {
        Item::Enum(ienum) => ienum,
        _ => Err(Error::new(Span::call_site(), "expected enum or match expression"))?,
    };

    Ok(ienum.into_token_stream())
}