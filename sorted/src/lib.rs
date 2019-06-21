extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse_macro_input, Error, Item};


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
        _ => Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ))?,
    };

    let variants: Vec<_> = ienum.variants.iter().map(|v| &v.ident).collect();

    for second in 1..variants.len() {
        if variants[second] < variants[second - 1] {
            // variants[second] is out of order, find the earliest spot it ought
            // to go and report that
            let mut first = second - 1;
            while first > 1 {
                if variants[second] < variants[first - 1] {
                    first -= 1;
                }
            }
            // Report the error at first
            Err(Error::new(
                variants[second].span(),
                format!(
                    "{} should sort before {}",
                    variants[second], variants[first]
                ),
            ))?;
        }
    }

    Ok(ienum.into_token_stream())
}