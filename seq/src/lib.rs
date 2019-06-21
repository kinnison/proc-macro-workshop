extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::quote;

use syn::{braced, parse_macro_input, Ident, LitInt, Result, Token};

use syn::parse::{Parse, ParseStream};

struct SeqMacroInner {
    ident: Ident,
    start: LitInt,
    end: LitInt,
    body: TokenStream2,
}

impl Parse for SeqMacroInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        let _in: Token![in] = input.parse()?;
        let start: LitInt = input.parse()?;
        let _dot2: Token![..] = input.parse()?;
        let end: LitInt = input.parse()?;
        let body;
        let _braces = braced!(body in input);
        let body: TokenStream2 = body.parse()?;
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
    let mut outputs: Vec<TokenStream2> = Vec::new();

    for n in input.start.value()..input.end.value() {
        outputs.push(replace_tokens(input.body.clone(), &input.ident, n)?);
    }

    Ok(quote! {#(#outputs)*})
}

fn replace_tokens(body: TokenStream2, ident: &Ident, n: u64) -> Result<TokenStream2> {
    use proc_macro2::{Group, Literal, Punct, TokenTree};
    fn replace_tokentree(tree: TokenTree, ident: &Ident, n: u64) -> Result<TokenTree> {
        Ok(match tree {
            TokenTree::Group(g) => TokenTree::Group(replace_group(g, ident, n)?),
            TokenTree::Ident(i) => replace_ident(i, ident, n)?,
            TokenTree::Punct(p) => TokenTree::Punct(p),
            TokenTree::Literal(l) => TokenTree::Literal(l),
        })
    }

    fn replace_group(g: Group, ident: &Ident, n: u64) -> Result<Group> {
        let span = g.span();
        let delim = g.delimiter();
        let stream = replace_tokens(g.stream(), ident, n)?;
        let mut ret = Group::new(delim, stream);
        ret.set_span(span);
        Ok(ret)
    }

    fn replace_ident(i: Ident, ident: &Ident, n: u64) -> Result<TokenTree> {
        if &i == ident {
            // Replace this with a literal number
            Ok(TokenTree::Literal(Literal::u64_unsuffixed(n)))
        } else {
            Ok(TokenTree::Ident(i))
        }
    }

    // First we're looking for Ident `#` Ident2
    // where Ident2 matches our incoming ident
    // if we get that, we paste the full span together into a token

    let toks: Vec<_> = body.into_iter().collect();
    let mut body: Vec<_> = Vec::new();
    enum ParseState {
        Waiting,
        FoundIdent(Ident),
        FoundHash(Ident, Punct),
    }
    use ParseState::*;
    let mut state = Waiting;
    for tok in toks {
        state = match state {
            Waiting => match tok {
                TokenTree::Ident(i) => FoundIdent(i),
                _ => {
                    body.push(tok);
                    Waiting
                }
            },
            FoundIdent(i) => match tok {
                TokenTree::Punct(p) => {
                    if p.as_char() == '#' {
                        FoundHash(i, p)
                    } else {
                        body.push(TokenTree::Ident(i));
                        body.push(TokenTree::Punct(p));
                        Waiting
                    }
                }
                TokenTree::Ident(i2) => {
                    body.push(TokenTree::Ident(i));
                    FoundIdent(i2)
                }
                _ => {
                    body.push(TokenTree::Ident(i));
                    body.push(tok);
                    Waiting
                }
            },
            FoundHash(i, p) => match tok {
                TokenTree::Ident(i2) => {
                    if &i2 == ident {
                        // paste
                        let newtok = Ident::new(&format!("{}{}", i, n), i.span());
                        body.push(TokenTree::Ident(newtok));
                        Waiting
                    } else {
                        body.push(TokenTree::Ident(i));
                        body.push(TokenTree::Punct(p));
                        FoundIdent(i2)
                    }
                }
                _ => {
                    body.push(TokenTree::Ident(i));
                    body.push(TokenTree::Punct(p));
                    body.push(tok);
                    Waiting
                }
            },
        }
    }

    match state {
        Waiting => {}
        FoundIdent(i) => body.push(TokenTree::Ident(i)),
        FoundHash(i, p) => {
            body.push(TokenTree::Ident(i));
            body.push(TokenTree::Punct(p));
        }
    }

    let bits: Result<Vec<_>> = body
        .into_iter()
        .map(|tt| replace_tokentree(tt, ident, n))
        .collect();
    let mut ret = TokenStream2::new();
    ret.extend(bits?);
    Ok(ret)
}
