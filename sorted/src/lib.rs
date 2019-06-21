extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::Span;
use proc_macro2::TokenStream as TS;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    visit_mut::{visit_expr_match_mut, visit_item_fn_mut, VisitMut},
    Error, ExprMatch, Ident, Item, ItemFn, Pat, Path,
};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let input: Item = parse_macro_input!(input as Item);

    (match sorted_impl(&input) {
        Ok(t) => t.into_token_stream(),
        Err(e) => {
            let e = e.to_compile_error();
            quote! {
                #e
                #input
            }
        }
    })
    .into()
}

fn sorted_impl(input: &Item) -> Result<impl ToTokens, Error> {
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
            while first > 0 {
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

struct SortedVisitor {
    errs: Vec<Error>,
}

impl SortedVisitor {
    pub fn new() -> SortedVisitor {
        SortedVisitor { errs: Vec::new() }
    }

    pub fn errors(&self) -> TS {
        let errs: Vec<_> = self.errs.iter().map(|e| e.to_compile_error()).collect();
        quote! {
            #(#errs)*
        }
    }
}

fn name_from_pattern(pat: &Pat) -> Option<Path> {
    match pat {
        Pat::Ident(i) => Some(i.ident.clone().into()),
        Pat::Struct(s) => Some(s.path.clone()),
        Pat::TupleStruct(ts) => Some(ts.path.clone()),
        Pat::Path(p) => Some(p.path.clone()),
        Pat::Wild(w) => Some(
            Ident::new(
                "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
                w.span(),
            )
            .into(),
        ),
        _ => None,
    }
}

fn path_to_string(path: &Path) -> String {
    let mut ret = String::new();
    if path.leading_colon.is_some() {
        ret.push_str("::");
    }

    for seg in path.segments.iter() {
        ret.push_str(&format!("{}::", seg.ident));
    }

    ret.pop();
    ret.pop();

    ret
}

impl VisitMut for SortedVisitor {
    fn visit_expr_match_mut(&mut self, expr: &mut ExprMatch) {
        visit_expr_match_mut(self, expr);
        let is_sorted = expr.attrs.iter().any(|att| att.path.is_ident("sorted"));
        if is_sorted {
            // We need to remove the sorted attribute
            expr.attrs
                .retain(|att| att.path.is_ident("sorted") == false);
            // Now we need to verify that each arm is in order...
            let all_pats: Vec<_> = expr
                .arms
                .iter()
                .flat_map(|arm| arm.pats.iter().map(|pat| (pat, name_from_pattern(pat))))
                .collect();

            for (pat, maybe_name) in all_pats.iter() {
                if maybe_name.is_none() {
                    let err = Error::new_spanned(pat, "unsupported by #[remain::sorted]");
                    self.errs.push(err);
                    // We probably just want to return now
                    return;
                }
            }
            let all_pats: Vec<_> = all_pats.iter().flat_map(|(_, n)| n).collect();

            let pat_names: Vec<_> = all_pats.iter().map(|p| path_to_string(p)).collect();

            for second in 1..all_pats.len() {
                for first in 0..second {
                    if pat_names[second] < pat_names[first] {
                        // Second should sort before first
                        let err = Error::new_spanned(
                            all_pats[second],
                            format!(
                                "{} should sort before {}",
                                pat_names[second], pat_names[first]
                            ),
                        );
                        self.errs.push(err);
                    }
                }
            }
        }
    }
}

#[proc_macro_attribute]
pub fn check(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);
    let mut visitor = SortedVisitor::new();

    visit_item_fn_mut(&mut visitor, &mut input);
    let errors = visitor.errors();

    (quote! {
        #input
        #errors
    })
    .into()
}
